#![allow(clippy::needless_return)]

use cursive::theme::{BorderStyle, Color, Palette, PaletteColor, Theme};
use cursive::view::{Margins, Nameable, Resizable, Scrollable};
use cursive::views::{LinearLayout, PaddedView, Panel, TextView};
use cursive::Cursive;
use cursive_tree_view::{Placement, TreeView};
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

fn view_database(siv: &mut Cursive, db: String) {
    siv.call_on_name("database_view", |view: &mut LinearLayout| {
        view.clear();
        view.add_child(TextView::new(db));
    });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);
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
    database_tree_view.set_on_submit(|siv: &mut Cursive, row| {
        let cb = siv.cb_sink().clone();
        let value = siv
            .call_on_name("db_tree", move |tree: &mut TreeView<String>| {
                return tree.borrow_item(row).unwrap().to_string();
            })
            .unwrap();

        cb.send(Box::new(|siv| view_database(siv, value))).unwrap();
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
