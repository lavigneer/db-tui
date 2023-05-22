use cursive::theme::{BorderStyle, Color, Palette, PaletteColor, Theme};

pub fn create_theme() -> Theme {
    let mut palette = Palette::default();
    palette[PaletteColor::Background] = Color::TerminalDefault;
    palette[PaletteColor::View] = Color::TerminalDefault;
    palette[PaletteColor::Primary] = Color::TerminalDefault;
    Theme {
        shadow: false,
        borders: BorderStyle::Simple,
        palette,
    }
}
