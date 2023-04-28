#![allow(clippy::needless_return)]

use cursive::reexports::enumset::enum_set;
use cursive::theme::{BorderStyle, Color, ColorStyle, Effect, Palette, PaletteColor, Style, Theme};
use cursive::view::{Margins, Nameable, Resizable, Scrollable};
use cursive::views::{LinearLayout, PaddedView, Panel, TextView};
use cursive::{CbSink, Cursive};
use cursive_tree_view::{Placement, TreeView};
use futures::stream::TryStreamExt;
use mongodb::bson::{doc, Document};
use mongodb::options::FindOptions;
use mongodb::Client;
use std::env;
use std::error::Error;

extern crate cursive_tree_view;

async fn create_database_tree_view(client: &Client) -> Result<TreeView<String>, Box<dyn Error>> {
    let databases = client.list_databases(None, None).await?;
    let mut tree = TreeView::new();

    for database in databases {
        let db_row = tree.insert_item(database.name.clone(), Placement::After, 0);
        match db_row {
            None => (),
            Some(row) => {
                let database = client.database(&database.name);
                let collection_names = database.list_collection_names(None).await?;
                for collection_name in collection_names {
                    tree.insert_item(collection_name, Placement::LastChild, row);
                }
            }
        }
    }
    return Ok(tree);
}

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
    return Ok(());
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
            let mut doc_tree_view = TreeView::new();
            doc_tree_view.disable();
            doc.iter().for_each(|(key, _value)| {
                doc_tree_view.insert_item(key.clone(), Placement::After, 0);
            });
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
    let mut palette = Palette::default();
    palette[PaletteColor::Background] = Color::TerminalDefault;
    palette[PaletteColor::View] = Color::TerminalDefault;
    palette[PaletteColor::Primary] = Color::TerminalDefault;
    siv.set_theme(Theme {
        shadow: false,
        borders: BorderStyle::Simple,
        palette,
    });
    let mut database_tree_view = create_database_tree_view(&client).await?;
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
    database_tree_view.set_on_submit(move |siv: &mut Cursive, row| {
        let client = client.clone();
        let cb = siv.cb_sink().clone();
        let (collection, db) = siv
            .call_on_name("db_tree", move |tree: &mut TreeView<String>| {
                let collection = tree.borrow_item(row).unwrap().clone();
                let db = tree
                    .borrow_item(tree.item_parent(row).unwrap())
                    .unwrap()
                    .clone();
                return (collection, db);
            })
            .unwrap();

        tokio::task::spawn(async move {
            load_database_collection(&client, db, collection, &cb)
                .await
                .unwrap();
        });
    });
    database_tree_layout.add_child(database_tree_view.with_name("db_tree").scrollable());

    let mut main_view = LinearLayout::horizontal();
    main_view.add_child(database_tree_layout);
    let database_view = LinearLayout::vertical();
    main_view.add_child(database_view.with_name("database_view"));
    siv.add_layer(Panel::new(main_view.full_screen().with_name("main")));

    // Starts the event loop.
    siv.run();

    return Ok(());
}
