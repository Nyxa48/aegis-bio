pub mod app;
pub mod calc;
pub mod db;
pub mod icons;
pub mod net;
pub mod ui;
pub mod ui_components;

pub use app::BiomonApp;

pub fn run_desktop() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("AEGIS-BIO — Astronaut Biomon HUD")
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([960.0, 540.0]),
        ..Default::default()
    };

    eframe::run_native(
        "AEGIS-BIO Biomon",
        native_options,
        Box::new(|cc| Ok(Box::new(BiomonApp::new(cc)))),
    )
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    let db_dir = app.internal_data_path();
    let mut native_options = eframe::NativeOptions::default();
    native_options.android_app = Some(app);

    let _ = eframe::run_native(
        "AEGIS-BIO Biomon",
        native_options,
        Box::new(move |cc| Ok(Box::new(BiomonApp::new_with_db_dir(cc, db_dir.clone())))),
    );
}
