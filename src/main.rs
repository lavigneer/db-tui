use cursive::views::{Dialog, TextView};
use cursive::theme::{Theme, Palette, BorderStyle};

fn main() {
    // Creates the cursive root - required for every application.
    let mut siv = cursive::default();
    siv.set_theme(Theme {
        shadow: false,
        borders: BorderStyle::Simple,
        palette: Palette::terminal_default()
    });
    // Creates a dialog with a single "Quit" button
    siv.add_layer(Dialog::around(TextView::new("Hello Dialog!"))
                         .title("Cursive")
                         .button("Quit", |s| s.quit()));

    // Starts the event loop.
    siv.run();
}

