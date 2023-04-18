use cursive::theme::{BorderStyle, Palette, Theme};
use cursive::view::{Nameable, Resizable, Scrollable};
use cursive::views::{LinearLayout, Panel};
use cursive_tree_view::{Placement, TreeView};
use mongodb::Client;
use std::env;
use std::error::Error;

extern crate cursive_tree_view;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let mongo_uri = &args[1];
    let client = Client::with_uri_str(mongo_uri).await?;

    let databases = client.list_databases(None, None).await?;
    // Creates the cursive root - required for every application.
    let mut siv = cursive::default();
    siv.set_theme(Theme {
        shadow: false,
        borders: BorderStyle::Simple,
        palette: Palette::terminal_default(),
    });

    let mut tree = TreeView::new();

    let mut main_list_view = LinearLayout::vertical();
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

    main_list_view.add_child(Panel::new(tree.with_name("Databases").scrollable()));
    siv.add_layer(main_list_view.full_screen());

    // Starts the event loop.
    siv.run();

    return Ok(());
}
