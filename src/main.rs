use cursive::reexports::enumset::enum_set;
use cursive::theme::{ColorStyle, Effect, Style};
use cursive::view::{Margins, Nameable, Resizable, Scrollable};
use cursive::views::{BoxedView, LinearLayout, PaddedView, Panel, TextView, ViewRef};
use cursive::{CbSink, Cursive};
use cursive_tree_view::{Placement, TreeView};
use db_tree::{DbTreeItem, DbTreeView};
use futures::stream::TryStreamExt;
use mongodb::bson::{doc, Bson, Document};
use mongodb::options::FindOptions;
use mongodb::Client;
use owning_ref::{OwningHandle, RcRef};
use std::env;
use std::error::Error;
use theme::create_theme;

extern crate cursive_tree_view;

mod collection_stats;
mod db_tree;
mod theme;

async fn load_database_collection(
    client: &Client,
    db: String,
    collection: String,
    cb: &CbSink,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let stats = client
        .database(&db)
        .run_command(doc! {"collStats": &collection }, None)
        .await?;
    let storage_size = stats.get_i32("storageSize").unwrap_or(0);
    let document_count = stats.get_i32("count").unwrap_or(0);
    let avg_document_size = stats.get_i32("avgObjSize").unwrap_or(0);
    let indexes_count = stats.get_i32("nindexes").unwrap_or(0);
    let total_index_size = stats.get_i32("totalIndexSize").unwrap_or(0);

    let coll = client.database(&db).collection::<Document>(&collection);

    let documents_cursor = coll
        .find(None, FindOptions::builder().limit(10).build())
        .await?;
    let documents: Vec<Document> = documents_cursor.try_collect().await?;

    cb.send(Box::new(move |siv| {
        show_database(
            siv,
            &collection,
            storage_size,
            document_count,
            avg_document_size,
            indexes_count,
            total_index_size,
            documents,
        )
    }))
    .unwrap();
    Ok(())
}

fn build_document_tree(
    tree: &mut TreeView<String>,
    parent_row: Option<usize>,
    field: (&String, &Bson),
) {
    let (key, value) = field;
    let placement = match parent_row {
        None => Placement::After,
        _ => Placement::LastChild,
    };
    let parent_row = parent_row.unwrap_or(0);
    let row = match value.to_owned() {
        Bson::Array(val) => {
            let row = tree.insert_item(format!("{}: Array", key), placement, parent_row);
            val.iter().enumerate().for_each(|(index, arr_val)| {
                build_document_tree(tree, row, (&index.to_string(), arr_val));
            });
            row
        }
        Bson::Document(val) => {
            let row = tree.insert_item(format!("{}: Object", key), placement, parent_row);
            val.iter()
                .for_each(|field| build_document_tree(tree, row, field));
            row
        }
        _ => tree.insert_item(format!("{}: {}", key, value), placement, parent_row),
    };
    if let Some(row) = row {
        tree.collapse_item(row);
    }
}

fn show_database(
    siv: &mut Cursive,
    collection: &String,
    storage_size: i32,
    document_count: i32,
    avg_document_size: i32,
    indexes_count: i32,
    total_index_size: i32,
    documents: Vec<Document>,
) {
    siv.call_on_name("database_view", |view: &mut LinearLayout| {
        view.clear();
        view.add_child(PaddedView::lrtb(
            0,
            0,
            0,
            1,
            TextView::new(collection).style(Style {
                color: ColorStyle::inherit_parent(),
                effects: enum_set!(Effect::Bold | Effect::Underline),
            }),
        ));
        let title_style = Style {
            color: ColorStyle::inherit_parent(),
            effects: enum_set!(Effect::Bold),
        };
        let stats_view = LinearLayout::horizontal()
            .child(PaddedView::lrtb(
                0,
                4,
                0,
                0,
                LinearLayout::vertical()
                    .child(TextView::new("Storage Size").style(title_style))
                    .child(TextView::new(format!("{} KB", storage_size / 1024))),
            ))
            .child(PaddedView::lrtb(
                0,
                4,
                0,
                0,
                LinearLayout::vertical()
                    .child(TextView::new("Documents").style(title_style))
                    .child(TextView::new(document_count.to_string())),
            ))
            .child(PaddedView::lrtb(
                0,
                4,
                0,
                0,
                LinearLayout::vertical()
                    .child(TextView::new("Avg Document Size").style(title_style))
                    .child(TextView::new(format!("{} KB", avg_document_size / 1024))),
            ))
            .child(PaddedView::lrtb(
                0,
                4,
                0,
                0,
                LinearLayout::vertical()
                    .child(TextView::new("Indexes").style(title_style))
                    .child(TextView::new(indexes_count.to_string())),
            ))
            .child(
                LinearLayout::vertical()
                    .child(TextView::new("Total Index Size").style(title_style))
                    .child(TextView::new(format!("{} KB", total_index_size / 1024))),
            );
        view.add_child(stats_view);

        let mut document_list_view = LinearLayout::vertical();
        documents.iter().for_each(|doc| {
            let mut doc_tree_view = TreeView::<String>::new();
            doc.iter()
                .for_each(|field| build_document_tree(&mut doc_tree_view, None, field));
            document_list_view.add_child(Panel::new(PaddedView::lrtb(1, 1, 1, 1, doc_tree_view)));
        });
        view.add_child(document_list_view);
    });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let mongo_uri = &args[1];
    let client = Client::with_uri_str(mongo_uri).await?;

    // Creates the cursive root - required for every application.
    let mut siv = cursive::default();
    siv.set_theme(create_theme());

    let mut db_tree_view = DbTreeView::new(siv.cb_sink().clone(), client.clone());
    db_tree_view.load_databases().await?;
    let mut database_tree_layout = LinearLayout::vertical();
    database_tree_layout.add_child(PaddedView::new(
        Margins {
            left: 1,
            top: 1,
            bottom: 1,
            right: 0,
        },
        TextView::new("Databases"),
    ));
    db_tree_view.set_on_submit(move |siv: &mut Cursive, db, collection| {
        let cb = siv.cb_sink().clone();
        let client = client.clone();
        tokio::task::spawn(async move {
            load_database_collection(&client, db, collection, &cb)
                .await
                .unwrap();
        });
    });
    let database_tree_view = db_tree_view.tree_view;
    database_tree_layout.add_child(OwningHandle::new_mut(database_tree_view));

    let mut main_view = LinearLayout::horizontal();
    main_view.add_child(database_tree_layout);
    let database_view = LinearLayout::vertical();
    main_view.add_child(database_view.with_name("database_view"));
    siv.add_layer(Panel::new(main_view.full_screen().with_name("main")));

    // Starts the event loop.
    siv.run();

    Ok(())
}
