pub mod app;
pub mod calc;
pub mod db;
pub mod icons;
pub mod net;
pub mod ui;
pub mod ui_components;

pub use app::HubApp;

pub fn run_desktop() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("AEGIS-BIO — Kutup | Görev Kontrol")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1024.0, 720.0]),
        ..Default::default()
    };

    eframe::run_native(
        "AEGIS-BIO Mission Control Hub",
        native_options,
        Box::new(|cc| Ok(Box::new(HubApp::new(cc)))),
    )
}
