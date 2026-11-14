use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use std::collections::VecDeque;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("A.S.T.R.O. - Terminal OS"),
        ..Default::default()
    };

    eframe::run_native(
        "AstroOS CRT Monitor",
        options,
        Box::new(|_cc| Box::new(AstroOsApp::default())),
    )
}

struct AstroOsApp {
    // Tema ve Ayarlar
    theme_color: egui::Color32,
    brightness: f32,
    show_settings: bool,

    // Sağlık Monitörü Verileri
    radiation: f32,
    oxygen: f32,
    isolation_pivot: f32,
    blood_pressure: f32,
    nitrogen_ratio: f32,
    hemoglobin: f32,
    heart_rate_base: f32,

    // Grafik Verileri (Zaman ve Geçmiş)
    time: f64,
    hr_history: VecDeque<[f64; 2]>,

    // Terminal Girdi/Çıktı
    terminal_logs: Vec<String>,
    terminal_input: String,
}

impl Default for AstroOsApp {
    fn default() -> Self {
        Self {
            theme_color: egui::Color32::from_rgb(0, 255, 60), // Klasik CRT Yeşili
            brightness: 1.0,
            show_settings: false,
            
            radiation: 12.5,
            oxygen: 98.0,
            isolation_pivot: 50.0,
            blood_pressure: 120.0,
            nitrogen_ratio: 78.0,
            hemoglobin: 14.5,
            heart_rate_base: 75.0,
            
            time: 0.0,
            hr_history: VecDeque::new(),
            
            terminal_logs: vec!["[SİSTEM] AstroOS v1.0.4 Yüklendi...".to_string(), "Yaşam destek ünitesi devrede.".to_string()],
            terminal_input: String::new(),
        }
    }
}

impl AstroOsApp {
    // Özel dikey progress bar çizim fonksiyonu
    fn custom_vertical_bar(&self, ui: &mut egui::Ui, value: f32, max: f32, label: &str) {
        let fraction = (value / max).clamp(0.0, 1.0);
        let current_color = self.crt_color();
        
        ui.horizontal(|ui| {
            // Dikdörtgen çizim alanı tahsisi
            let size = egui::vec2(40.0, 180.0);
            let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());

            // Arka plan kutusu (çerçeve)
            ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(2.0, current_color));

            // Dolgu oranı hesabı ve çizimi
            let fill_height = fraction * rect.height();
            let fill_rect = egui::Rect::from_min_max(
                egui::pos2(rect.min.x, rect.max.y - fill_height),
                rect.max
            );
            ui.painter().rect_filled(fill_rect, 0.0, current_color);

            // Sağ tarafta dev yüzde ve isim etiketi
            ui.vertical(|ui| {
                ui.add_space(size.y / 2.0 - 20.0); // Ortalamak için
                ui.label(
                    egui::RichText::new(format!("%{:.1}", fraction * 100.0))
                        .size(32.0) // BÜYÜK yüzde metni
                        .monospace()
                        .color(current_color),
                );
                ui.label(
                    egui::RichText::new(label)
                        .size(16.0)
                        .monospace()
                        .color(current_color),
                );
            });
        });
    }

    // Parlaklık ve RGBA ile hesaplanmış nihai tema rengi
    fn crt_color(&self) -> egui::Color32 {
        let r = (self.theme_color.r() as f32 * self.brightness).clamp(0.0, 255.0) as u8;
        let g = (self.theme_color.g() as f32 * self.brightness).clamp(0.0, 255.0) as u8;
        let b = (self.theme_color.b() as f32 * self.brightness).clamp(0.0, 255.0) as u8;
        let a = self.theme_color.a();
        egui::Color32::from_rgba_premultiplied(r, g, b, a)
    }

    fn apply_crt_theme(&self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        let crt_c = self.crt_color();
        
        // Sınırları zorla: Tüm yuvarlamaları (rounding) 0 yap, arka planı zifiri siyah yap.
        visuals.window_rounding = egui::Rounding::ZERO;
        visuals.menu_rounding = egui::Rounding::ZERO;
        visuals.widgets.noninteractive.rounding = egui::Rounding::ZERO;
        visuals.widgets.inactive.rounding = egui::Rounding::ZERO;
        visuals.widgets.hovered.rounding = egui::Rounding::ZERO;
        visuals.widgets.active.rounding = egui::Rounding::ZERO;
        
        visuals.window_fill = egui::Color32::BLACK;
        visuals.panel_fill = egui::Color32::BLACK;
        visuals.override_text_color = Some(crt_c);
        
        ctx.set_visuals(visuals);

        // Tüm metinleri Monospace formatına zorla
        let mut style = (*ctx.style()).clone();
        style.text_styles.iter_mut().for_each(|(_, font_id)| {
            font_id.family = egui::FontFamily::Monospace;
        });
        ctx.set_style(style);
    }
}

impl eframe::App for AstroOsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_crt_theme(ctx);

        // Canlı veri simülasyonu (grafik için zamanı ilerlet)
        self.time += ctx.input(|i| i.stable_dt) as f64;
        let current_hr = self.heart_rate_base as f64 + (self.time * 5.0).sin() * 10.0;
        self.hr_history.push_back([self.time, current_hr]);
        if self.hr_history.len() > 300 {
            self.hr_history.pop_front();
        }
        ctx.request_repaint(); // Her frame'i güncelle (animasyon için)

        // --- ALT PANEL: CMD Terminal / Input-Output ---
        egui::TopBottomPanel::bottom("cmd_panel")
            .exact_height(150.0)
            .show(ctx, |ui| {
                ui.heading(">_ ASTRO TERMINAL (I/O)");
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(80.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for log in &self.terminal_logs {
                            ui.label(log);
                        }
                    });
                
                ui.horizontal(|ui| {
                    ui.label("root@astro:~# ");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.terminal_input)
                            .desired_width(f32::INFINITY)
                            .frame(false), // Çerçevesiz CMD stili
                    );
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        if self.terminal_input == "settings" {
                            self.show_settings = true;
                        } else if !self.terminal_input.is_empty() {
                            let log = format!("> {}", self.terminal_input);
                            self.terminal_logs.push(log);
                            self.terminal_logs.push("[SİSTEM] Bilinmeyen Komut".to_string());
                        }
                        self.terminal_input.clear();
                        response.request_focus();
                    }
                });
            });

        // --- SOL PANEL: Çoklu Sağlık Monitörü ---
        egui::SidePanel::left("health_panel")
            .exact_width(400.0)
            .show(ctx, |ui| {
                ui.heading("=== HEALTH MONITOR ===");
                ui.separator();

                ui.horizontal(|ui| {
                    // Dikey Çubuklar
                    self.custom_vertical_bar(ui, self.oxygen, 100.0, "O2 SEVİYESİ");
                    ui.add_space(20.0);
                    self.custom_vertical_bar(ui, self.radiation, 100.0, "RADYASYON");
                });

                ui.add_space(20.0);
                ui.label("--- KAN SAYIMI ---");
                ui.add(egui::Slider::new(&mut self.blood_pressure, 60.0..=180.0).text("Basınç (mmHg)"));
                ui.add(egui::Slider::new(&mut self.nitrogen_ratio, 0.0..=100.0).text("Nitrojen Doygunluğu"));
                ui.add(egui::Slider::new(&mut self.hemoglobin, 10.0..=20.0).text("Hemoglobin (g/dL)"));
                
                ui.add_space(30.0);
                
                // Dev Panik Butonu
                let panic_btn = egui::Button::new(
                    egui::RichText::new("!!! PANİK BUTONU !!!")
                        .size(24.0)
                        .color(egui::Color32::BLACK)
                )
                .fill(egui::Color32::RED)
                .min_size(egui::vec2(ui.available_width(), 60.0));

                if ui.add(panic_btn).clicked() {
                    self.terminal_logs.push("!!! ACİL DURUM PROTOKOLÜ BAŞLATILDI !!!".to_string());
                }
            });

        // --- ORTA PANEL: Grafik ve Pivot Modeli ---
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("=== BİYOMETRİK ZAMAN ÇİZELGESİ ===");
            ui.separator();

            // Zaman Çizelgesi (Kalp Atışı Plot)
            let hr_points: PlotPoints = self.hr_history.iter().cloned().collect();
            let hr_line = Line::new(hr_points)
                .color(self.crt_color())
                .width(2.0);

            Plot::new("hr_plot")
                .view_aspect(2.0)
                .show_axes([true, true])
                .allow_drag(false)
                .allow_zoom(false)
                .show(ui, |plot_ui| plot_ui.line(hr_line));

            ui.add_space(20.0);
            
            ui.heading("=== İZOLASYON PİVOT MODELİ ===");
            ui.separator();
            ui.label(format!("Mevcut Eksen Yönelimi: {:.2}°", self.isolation_pivot));
            ui.add(egui::Slider::new(&mut self.isolation_pivot, 0.0..=360.0).text("Manuel Kalibrasyon"));
            
            // Temsili Pivot Görselleştirmesi
            let (rect, _) = ui.allocate_exact_size(egui::vec2(200.0, 200.0), egui::Sense::hover());
            let center = rect.center();
            let angle = self.isolation_pivot.to_radians();
            let end_point = egui::pos2(
                center.x + angle.cos() * 80.0,
                center.y + angle.sin() * 80.0,
            );
            ui.painter().circle_stroke(center, 80.0, egui::Stroke::new(1.0, self.crt_color()));
            ui.painter().line_segment([center, end_point], egui::Stroke::new(3.0, egui::Color32::RED));
        });

        // --- AYARLAR PENCERESİ (Bağımsız Pencere) ---
        if self.show_settings {
            egui::Window::new("Ayar Konsolu")
                .collapsible(false)
                .resizable(false)
                .open(&mut self.show_settings)
                .show(ctx, |ui| {
                    ui.label("Sistem Tema Rengi (RGBA):");
                    ui.color_edit_button_srgba(&mut self.theme_color);
                    
                    ui.add_space(10.0);
                    ui.label("CRT Parlaklığı:");
                    ui.add(egui::Slider::new(&mut self.brightness, 0.1..=3.0));
                });
        }
    }
}