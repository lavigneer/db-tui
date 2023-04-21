#![allow(clippy::needless_return)]

use cursive::theme::{BorderStyle, Color, Palette, PaletteColor, Theme};
use cursive::view::{Margins, Nameable, Resizable, Scrollable};
use cursive::views::{DebugView, LinearLayout, PaddedView, Panel, TextView};
use cursive::{CbSink, Cursive};
use cursive_tree_view::{Placement, TreeView};
use mongodb::bson::doc;
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
        .run_command(doc! {"collStats": &collection, "scale": 1024}, None)
        .await?;
    let size = stats.get_i32("size").unwrap();

    cb.send(Box::new(move |siv| show_database(siv, &collection, size)))
        .unwrap();
    return Ok(());
}

fn show_database(siv: &mut Cursive, collection: &String, size: i32) {
    siv.call_on_name("database_view", |view: &mut LinearLayout| {
        view.clear();
        view.add_child(TextView::new(collection));
        view.add_child(TextView::new(format!("{} KB", size)));
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
