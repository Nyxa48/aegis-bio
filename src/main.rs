use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 350.0])
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "Astronot Yaşam Monitörü",
        options,
        Box::new(|_cc| Box::new(AstronautMonitor::default())),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AlertLevel {
    Critical,
    Warning,
}

struct AstronautMonitor {
    heart_rate: f32,
    oxygen_level: f32,
    suit_pressure: f32,
    temperature: f32,
}

impl Default for AstronautMonitor {
    fn default() -> Self {
        Self {
            heart_rate: 75.0,
            oxygen_level: 98.0,
            suit_pressure: 1.0,
            temperature: 22.0,
        }
    }
}

impl AstronautMonitor {
    fn active_alerts(&self) -> Vec<(AlertLevel, &'static str)> {
        let mut alerts = Vec::new();
        if self.oxygen_level < 20.0 {
            alerts.push((AlertLevel::Critical, "⚠ KRİTİK: DÜŞÜK OKSİJEN!"));
        }
        if self.suit_pressure < 0.7 {
            alerts.push((AlertLevel::Critical, "⚠ KRİTİK: BASINÇ DÜŞÜŞÜ!"));
        }
        if self.heart_rate > 140.0 {
            alerts.push((AlertLevel::Warning, "⚠ DİKKAT: YÜKSEK NABIZ!"));
        }
        alerts
    }
}

impl eframe::App for AstronautMonitor {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("AEGIS-BIO // Astronot Yaşam Monitörü");
            ui.add_space(10.0);

            // Heart Rate
            ui.horizontal(|ui| {
                ui.label("Nabız:");
                ui.strong(format!("{:.0} BPM", self.heart_rate));
            });
            ui.add(egui::Slider::new(&mut self.heart_rate, 40.0..=180.0).text("BPM"));
            ui.add_space(6.0);

            // Oxygen Level
            ui.horizontal(|ui| {
                ui.label("Oksijen Seviyesi:");
                ui.strong(format!("{:.1}%", self.oxygen_level));
            });
            let o2_color = if self.oxygen_level > 20.0 {
                egui::Color32::LIGHT_BLUE
            } else {
                egui::Color32::RED
            };
            ui.add(
                egui::ProgressBar::new(self.oxygen_level / 100.0)
                    .fill(o2_color)
                    .show_percentage(),
            );
            ui.add(egui::Slider::new(&mut self.oxygen_level, 0.0..=100.0).text("%"));
            ui.add_space(6.0);

            // Suit Pressure
            ui.horizontal(|ui| {
                ui.label("Kıyafet Basıncı:");
                ui.strong(format!("{:.2} atm", self.suit_pressure));
            });
            ui.add(egui::Slider::new(&mut self.suit_pressure, 0.0..=2.0).text("atm"));
            ui.add_space(6.0);

            // Internal Temperature
            let temp_color = if (15.0..=30.0).contains(&self.temperature) {
                egui::Color32::GREEN
            } else {
                egui::Color32::RED
            };
            ui.horizontal(|ui| {
                ui.label("İç Sıcaklık:");
                ui.colored_label(temp_color, format!("{:.1} °C", self.temperature));
            });
            ui.add(egui::Slider::new(&mut self.temperature, -20.0..=60.0).text("°C"));

            ui.separator();
            ui.add_space(4.0);

            // Critical Alert Panel
            let alerts = self.active_alerts();
            if alerts.is_empty() {
                ui.colored_label(egui::Color32::GREEN, "Tüm sistemler nominal.");
            } else {
                for (level, msg) in alerts {
                    let color = match level {
                        AlertLevel::Critical => egui::Color32::RED,
                        AlertLevel::Warning => egui::Color32::YELLOW,
                    };
                    ui.colored_label(color, msg);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_vitals_have_no_alerts() {
        let monitor = AstronautMonitor::default();
        assert!(monitor.active_alerts().is_empty());
    }

    #[test]
    fn critical_and_warning_alerts_trigger_as_expected() {
        let monitor = AstronautMonitor {
            heart_rate: 155.0,
            oxygen_level: 18.0,
            suit_pressure: 0.5,
            temperature: 22.0,
        };
        let alerts = monitor.active_alerts();
        assert_eq!(alerts.len(), 3);
        assert_eq!(alerts[0].0, AlertLevel::Critical);
        assert_eq!(alerts[1].0, AlertLevel::Critical);
        assert_eq!(alerts[2].0, AlertLevel::Warning);
    }
}
