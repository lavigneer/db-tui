use std::error::Error;
use std::env;
use cursive::theme::{BorderStyle, Palette, Theme};
use cursive::view::Resizable;
use cursive::views::{TextView, LinearLayout};
use mongodb::Client;

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

    let mut main_list_view = LinearLayout::vertical();
    for database in databases {
        main_list_view = main_list_view.child( TextView::new(&database.name));
    }

    siv.add_layer(main_list_view.full_screen());

    // Starts the event loop.
    siv.run();

    return Ok(());
}
