pub mod hub_data;

use eframe::egui::{self, FontData, FontDefinitions, FontFamily, FontId};

/// Installs Windows geometric sans (`Segoe UI` / `Segoe UI Semibold`) and tabular
/// monospace (`Cascadia Mono` / `Consolas`) fonts once at startup for both binaries.
pub fn install_aerospace_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let try_load = |path: &str| -> Option<FontData> {
        std::fs::read(path).ok().map(FontData::from_owned)
    };

    if let Some(data) = try_load("C:\\Windows\\Fonts\\segoeui.ttf") {
        fonts.font_data.insert("segoe_ui".to_owned(), data);
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "segoe_ui".to_owned());
    }
    if let Some(data) = try_load("C:\\Windows\\Fonts\\seguisb.ttf") {
        fonts.font_data.insert("segoe_sb".to_owned(), data);
        fonts
            .families
            .entry(FontFamily::Name("heading".into()))
            .or_default()
            .push("segoe_sb".to_owned());
        if let Some(prop) = fonts.families.get(&FontFamily::Proportional).cloned() {
            fonts
                .families
                .entry(FontFamily::Name("heading".into()))
                .or_default()
                .extend(prop);
        }
    }
    if let Some(data) = try_load("C:\\Windows\\Fonts\\CascadiaMono.ttf")
        .or_else(|| try_load("C:\\Windows\\Fonts\\consola.ttf"))
    {
        fonts.font_data.insert("aero_mono".to_owned(), data);
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, "aero_mono".to_owned());
    }

    ctx.set_fonts(fonts);
}

/// Returns the semibold heading `FontId` configured by `install_aerospace_fonts`.
pub fn heading_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("heading".into()))
}
