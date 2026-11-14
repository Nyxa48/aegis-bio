use eframe::egui::{
    self, Align, Align2, Color32, FontId, Key, Layout, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2,
};
use egui_plot::{Line, Plot, PlotPoints};
use std::collections::VecDeque;

// ───────────────────────── Giriş ─────────────────────────
fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        // Kol tableti gibi DİKEY ekran
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([420.0, 860.0])
            .with_min_inner_size([360.0, 700.0])
            .with_title("AEGIS-BIO – Kol Tableti"),
        ..Default::default()
    };
    eframe::run_native(
        "AEGIS-BIO Kol Tableti",
        options,
        Box::new(|cc| Box::new(App::new(cc))),
    )
}

// ───────────────────────── Renk paleti ─────────────────────────
#[derive(Clone, Copy)]
struct Pal {
    bg: Color32,
    panel: Color32,
    panel_hi: Color32,
    edge: Color32,
    text: Color32,
    dim: Color32,
    accent: Color32,
    ok: Color32,
    warn: Color32,
    danger: Color32,
    bar_bg: Color32,
}

/// RGBA (unmultiplied) renge parlaklık çarpanı uygular.
fn sc(c: [u8; 4], k: f32) -> Color32 {
    let f = |v: u8| ((v as f32) * k).clamp(0.0, 255.0) as u8;
    Color32::from_rgba_unmultiplied(f(c[0]), f(c[1]), f(c[2]), c[3])
}

#[derive(PartialEq, Clone, Copy)]
enum Page {
    Home,
    Health,
    Settings,
}

// ───────────────────────── Uygulama durumu ─────────────────────────
struct App {
    page: Page,

    // Ayarlar (RGBA)
    brightness: f32, // 0.3 .. 1.5
    c_bg: [u8; 4],
    c_panel: [u8; 4],
    c_accent: [u8; 4],

    // Sağlık verileri
    bpm: f32,
    bpm_base: f32,
    oxygen: f32,
    radiation: f32,
    pressure: f32,
    nitrogen: f32,
    hemoglobin: f32,
    pivot_deg: f32,
    panic: bool,

    // Grafik
    hr_hist: VecDeque<[f64; 2]>,
    last_sample: f64,

    // Terminal
    logs: Vec<String>,
    input: String,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Eldivenle kullanım için büyük dokunma alanları
        cc.egui_ctx.style_mut(|s| {
            s.spacing.interact_size.y = 40.0;
            s.spacing.item_spacing = Vec2::new(8.0, 8.0);
            s.spacing.button_padding = Vec2::new(12.0, 8.0);
        });
        Self {
            page: Page::Home,
            brightness: 1.0,
            c_bg: [10, 16, 28, 255],
            c_panel: [22, 32, 50, 240],
            c_accent: [70, 190, 255, 255],
            bpm: 72.0,
            bpm_base: 72.0,
            oxygen: 98.0,
            radiation: 12.5,
            pressure: 120.0,
            nitrogen: 78.0,
            hemoglobin: 14.5,
            pivot_deg: 50.0,
            panic: false,
            hr_hist: VecDeque::new(),
            last_sample: 0.0,
            logs: vec![
                "[SİSTEM] AegisOS kol tableti hazır.".into(),
                "[SİSTEM] Komutlar için 'help' yazın.".into(),
            ],
            input: String::new(),
        }
    }

    fn pal(&self) -> Pal {
        let k = self.brightness;
        let panel = sc(self.c_panel, k);
        Pal {
            bg: sc(self.c_bg, k),
            panel,
            panel_hi: sc(
                [
                    self.c_panel[0].saturating_add(14),
                    self.c_panel[1].saturating_add(18),
                    self.c_panel[2].saturating_add(26),
                    self.c_panel[3],
                ],
                k,
            ),
            edge: sc([64, 88, 124, 255], k),
            text: sc([228, 236, 248, 255], k),
            dim: sc([142, 162, 188, 255], k),
            accent: sc(self.c_accent, k),
            ok: sc([72, 212, 142, 255], k),
            warn: sc([255, 186, 62, 255], k),
            danger: sc([255, 82, 82, 255], k),
            bar_bg: sc([12, 18, 30, 255], k),
        }
    }

    fn apply_visuals(&self, ctx: &egui::Context, p: &Pal) {
        let mut v = egui::Visuals::dark();
        v.panel_fill = p.bg;
        v.window_fill = p.panel;
        v.extreme_bg_color = p.bar_bg;
        v.override_text_color = Some(p.text);
        v.selection.bg_fill = p.accent;
        v.selection.stroke = Stroke::new(1.0, p.text);
        v.widgets.inactive.bg_fill = p.edge;
        v.widgets.inactive.weak_bg_fill = p.panel_hi;
        v.widgets.hovered.weak_bg_fill = p.edge;
        v.widgets.active.weak_bg_fill = p.accent;
        v.widgets.inactive.fg_stroke = Stroke::new(1.0, p.text);
        ctx.set_visuals(v);
    }

    fn log(&mut self, s: impl Into<String>) {
        self.logs.push(s.into());
        if self.logs.len() > 200 {
            self.logs.remove(0);
        }
    }

    // ── Terminal komutları ──
    fn run(&mut self, cmd: String) {
        let cmd = cmd.trim().to_lowercase();
        if cmd.is_empty() {
            return;
        }
        self.log(format!("> {cmd}"));
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let num = parts.get(1).and_then(|s| s.parse::<f32>().ok());
        match parts[0] {
            "help" => {
                for l in [
                    "help            komut listesi",
                    "status          anlık değerler",
                    "health          sağlık ekranı",
                    "home            ana ekran",
                    "settings        ayarlar",
                    "panic / cancel  acil durum aç/kapat",
                    "bpm <n>         taban nabız",
                    "o2 <n>          oksijen %",
                    "rad <n>         radyasyon %",
                    "clear           terminali temizle",
                ] {
                    self.log(l);
                }
            }
            "status" => {
                let s = format!(
                    "BPM {:.0} | O2 %{:.1} | RAD %{:.1} | BP {:.0} | N2 %{:.0} | Hb {:.1}",
                    self.bpm, self.oxygen, self.radiation, self.pressure, self.nitrogen, self.hemoglobin
                );
                self.log(s);
            }
            "health" => self.page = Page::Health,
            "home" => self.page = Page::Home,
            "settings" => self.page = Page::Settings,
            "panic" => {
                self.panic = true;
                self.log("[ALARM] ACİL DURUM AKTİF – Görev Kontrol bilgilendirildi.");
            }
            "cancel" => {
                self.panic = false;
                self.log("[SİSTEM] Acil durum iptal edildi.");
            }
            "bpm" => match num {
                Some(n) => {
                    self.bpm_base = n.clamp(30.0, 200.0);
                    self.log(format!("[OK] Taban nabız {:.0}", self.bpm_base));
                }
                None => self.log("[HATA] kullanım: bpm 80"),
            },
            "o2" => match num {
                Some(n) => {
                    self.oxygen = n.clamp(0.0, 100.0);
                    self.log(format!("[OK] O2 %{:.1}", self.oxygen));
                }
                None => self.log("[HATA] kullanım: o2 97"),
            },
            "rad" => match num {
                Some(n) => {
                    self.radiation = n.clamp(0.0, 100.0);
                    self.log(format!("[OK] Radyasyon %{:.1}", self.radiation));
                }
                None => self.log("[HATA] kullanım: rad 20"),
            },
            "clear" => self.logs.clear(),
            _ => self.log("[HATA] bilinmeyen komut ('help')"),
        }
    }

    fn simulate(&mut self, t: f64) {
        let target = self.bpm_base
            + 4.0 * (t * 0.9).sin() as f32
            + 2.0 * (t * 2.3).sin() as f32
            + if self.panic { 38.0 } else { 0.0 };
        self.bpm += (target - self.bpm) * 0.05;
        if t - self.last_sample > 0.1 {
            self.last_sample = t;
            self.hr_hist.push_back([t, self.bpm as f64]);
            while self.hr_hist.front().map_or(false, |p| p[0] < t - 15.0) {
                self.hr_hist.pop_front();
            }
        }
    }
}

// ───────────────────────── Çizim yardımcıları ─────────────────────────
fn heart(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    let n = 56;
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(c, color);
    for i in 0..n {
        let t = i as f32 / n as f32 * std::f32::consts::TAU;
        let x = 16.0 * t.sin().powi(3);
        let y = 13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
        mesh.colored_vertex(
            Pos2::new(c.x + x * size / 34.0, c.y - (y + 2.5) * size / 34.0),
            color,
        );
    }
    for i in 0..n {
        mesh.add_triangle(0, 1 + i as u32, 1 + ((i + 1) % n) as u32);
    }
    painter.add(egui::Shape::mesh(mesh));
}

fn card(ui: &mut Ui, p: &Pal, title: &str, add: impl FnOnce(&mut Ui)) {
    egui::Frame::none()
        .fill(p.panel)
        .stroke(Stroke::new(1.0, p.edge))
        .rounding(14.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).size(12.0).strong().color(p.dim));
            ui.add_space(4.0);
            add(ui);
        });
}

/// Dikey dikdörtgen progress bar
fn vbar(ui: &mut Ui, p: &Pal, frac: f32, color: Color32, size: Vec2) {
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 6.0, p.bar_bg);
    painter.rect_stroke(rect, 6.0, Stroke::new(1.5, color));
    let inner = rect.shrink(4.0);
    let h = inner.height() * frac.clamp(0.0, 1.0);
    let fill = Rect::from_min_max(Pos2::new(inner.left(), inner.bottom() - h), inner.right_bottom());
    painter.rect_filled(fill, 3.0, color);
}

/// Dikey bar + sağında BÜYÜK yüzde yazısı
fn metric(ui: &mut Ui, p: &Pal, title: &str, value: f32, color: Color32) {
    card(ui, p, title, |ui| {
        ui.horizontal(|ui| {
            vbar(ui, p, value / 100.0, color, Vec2::new(34.0, 130.0));
            ui.vertical(|ui| {
                ui.add_space(38.0);
                ui.label(RichText::new(format!("%{value:.1}")).size(30.0).strong().color(color));
            });
        });
    });
}

fn blood_row(ui: &mut Ui, p: &Pal, name: &str, val: &mut f32, range: std::ops::RangeInclusive<f32>, unit: &str, color: Color32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(name).color(p.dim));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(format!("{:.1} {unit}", *val)).size(22.0).strong().color(color));
        });
    });
    ui.spacing_mut().slider_width = ui.available_width() - 8.0;
    ui.add(egui::Slider::new(val, range).show_value(false).trailing_fill(true));
}

fn level_color(p: &Pal, v: f32, warn_low: f32, danger_low: f32) -> Color32 {
    if v < danger_low {
        p.danger
    } else if v < warn_low {
        p.warn
    } else {
        p.ok
    }
}

// ───────────────────────── Ana döngü ─────────────────────────
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let t = ctx.input(|i| i.time);
        self.simulate(t);
        let p = self.pal();
        self.apply_visuals(ctx, &p);
        ctx.request_repaint();

        // ── Üst çubuk ──
        egui::TopBottomPanel::top("header")
            .exact_height(54.0)
            .frame(egui::Frame::none().fill(p.panel).inner_margin(egui::Margin::symmetric(10.0, 7.0)))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    if self.page != Page::Home {
                        if ui.add(egui::Button::new(RichText::new("‹ GERİ").strong())).clicked() {
                            self.page = Page::Home;
                        }
                    } else {
                        ui.label(RichText::new("AEGIS-BIO").size(18.0).strong().color(p.accent));
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.add(egui::Button::new(RichText::new("AYARLAR").strong())).clicked() {
                            self.page = Page::Settings;
                        }
                        let s = t as u64;
                        ui.label(
                            RichText::new(format!("ORBIT_IVA  {:02}:{:02}", (s / 60) % 60, s % 60))
                                .size(12.0)
                                .color(p.dim),
                        );
                    });
                });
            });

        // ── İçerik ──
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(p.bg).inner_margin(egui::Margin::same(12.0)))
            .show(ctx, |ui| match self.page {
                Page::Home => self.home(ui, ctx, &p, t),
                Page::Health => {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| self.health_page(ui, &p));
                }
                Page::Settings => self.settings_page(ui, &p),
            });
    }
}

// ───────────────────────── Sayfalar ─────────────────────────
impl App {
    fn home(&mut self, ui: &mut Ui, ctx: &egui::Context, p: &Pal, t: f64) {
        let screen = ctx.screen_rect();
        let w = screen.width();
        let h = screen.height();
        let cy = h / 2.0; // ekranın tam dikey ortası
        let m = 12.0;

        // 1) Sağlık butonu (atan kalp + BPM)
        let top = 54.0 + m;
        let hb_h = (cy - 62.0 - top).clamp(110.0, 230.0);
        let hb = Rect::from_min_size(Pos2::new(m, top), Vec2::new(w - 2.0 * m, hb_h));
        let resp = ui.allocate_rect(hb, Sense::click());
        let painter = ui.painter().clone();
        painter.rect_filled(hb, 18.0, if resp.hovered() { p.panel_hi } else { p.panel });
        painter.rect_stroke(hb, 18.0, Stroke::new(2.0, p.accent));
        let phase = ((t * self.bpm as f64 / 60.0) % 1.0) as f32;
        let pulse = (1.0 - phase * 3.0).max(0.0).powi(2);
        let hs = (hb.height() * 0.5).min(110.0) * (1.0 + 0.2 * pulse);
        heart(&painter, Pos2::new(hb.left() + hb.width() * 0.27, hb.center().y - 6.0), hs, p.danger);
        painter.text(
            Pos2::new(hb.left() + hb.width() * 0.50, hb.center().y - 14.0),
            Align2::LEFT_CENTER,
            format!("{:.0}", self.bpm),
            FontId::proportional(54.0),
            p.text,
        );
        painter.text(
            Pos2::new(hb.left() + hb.width() * 0.50, hb.center().y + 26.0),
            Align2::LEFT_CENTER,
            "BPM",
            FontId::proportional(18.0),
            p.dim,
        );
        painter.text(
            Pos2::new(hb.center().x, hb.bottom() - 14.0),
            Align2::CENTER_CENTER,
            "SAĞLIK  •  detaylar için dokun",
            FontId::proportional(13.0),
            p.dim,
        );
        if resp.clicked() {
            self.page = Page::Health;
        }

        // 2) PANİK butonu – ekranın tam ortasında, yatay
        let pb = Rect::from_center_size(Pos2::new(w / 2.0, cy), Vec2::new(w - 2.0 * m, 84.0));
        let presp = ui.allocate_rect(pb, Sense::click());
        let blink = if self.panic { 0.55 + 0.45 * (t * 8.0).sin().abs() as f32 } else { 1.0 };
        let [r, g, b, _] = p.danger.to_srgba_unmultiplied();
        let fill = Color32::from_rgb((r as f32 * blink) as u8, (g as f32 * blink) as u8, (b as f32 * blink) as u8);
        painter.rect_filled(pb, 18.0, fill);
        painter.rect_stroke(pb, 18.0, Stroke::new(3.0, Color32::from_white_alpha(200)));
        painter.text(
            pb.center(),
            Align2::CENTER_CENTER,
            if self.panic { "ACİL DURUM AKTİF\niptal için dokun" } else { "PANİK" },
            FontId::proportional(if self.panic { 20.0 } else { 36.0 }),
            Color32::WHITE,
        );
        if presp.clicked() {
            self.panic = !self.panic;
            if self.panic {
                self.log("[ALARM] PANİK butonuna basıldı – Görev Kontrol bilgilendirildi.");
            } else {
                self.log("[SİSTEM] Acil durum iptal edildi.");
            }
        }

        // 3) Hızlı durum çipleri
        let term_h = (h * 0.32).max(200.0);
        let term_top = h - term_h - m;
        let chips_top = pb.bottom() + 10.0;
        if term_top - chips_top > 48.0 {
            let cw = (w - 2.0 * m - 16.0) / 3.0;
            let items = [
                ("O2", format!("%{:.0}", self.oxygen), level_color(p, self.oxygen, 95.0, 90.0)),
                ("RAD", format!("%{:.0}", self.radiation), level_color(p, 100.0 - self.radiation, 50.0, 25.0)),
                ("BP", format!("{:.0}", self.pressure), p.accent),
            ];
            for (i, (name, val, col)) in items.iter().enumerate() {
                let r = Rect::from_min_size(
                    Pos2::new(m + i as f32 * (cw + 8.0), chips_top),
                    Vec2::new(cw, (term_top - chips_top - 8.0).min(64.0)),
                );
                painter.rect_filled(r, 12.0, p.panel);
                painter.rect_stroke(r, 12.0, Stroke::new(1.0, p.edge));
                painter.text(Pos2::new(r.center().x, r.top() + 14.0), Align2::CENTER_CENTER, *name, FontId::proportional(12.0), p.dim);
                painter.text(Pos2::new(r.center().x, r.bottom() - 20.0), Align2::CENTER_CENTER, val, FontId::proportional(24.0), *col);
            }
        }

        // 4) Terminal (G/Ç) – ekranın altında
        let tr = Rect::from_min_size(Pos2::new(m, term_top), Vec2::new(w - 2.0 * m, term_h));
        ui.allocate_ui_at_rect(tr, |ui| self.terminal(ui, p));
    }

    fn terminal(&mut self, ui: &mut Ui, p: &Pal) {
        egui::Frame::none()
            .fill(p.bar_bg)
            .stroke(Stroke::new(1.0, p.accent))
            .rounding(12.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.set_min_size(Vec2::new(ui.available_width(), ui.available_height() - 4.0));
                ui.label(RichText::new(">_ TERMİNAL (G/Ç)").size(12.0).strong().color(p.accent));
                let h = (ui.available_height() - 44.0).max(30.0);
                egui::ScrollArea::vertical()
                    .max_height(h)
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for l in &self.logs {
                            let col = if l.starts_with("[ALARM]") || l.starts_with("[HATA]") {
                                p.danger
                            } else if l.starts_with('>') {
                                p.accent
                            } else {
                                p.text
                            };
                            ui.label(RichText::new(l).monospace().size(12.0).color(col));
                        }
                    });
                let r = ui.add(
                    egui::TextEdit::singleline(&mut self.input)
                        .desired_width(f32::INFINITY)
                        .font(FontId::monospace(14.0))
                        .hint_text("komut: help"),
                );
                if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    let cmd = std::mem::take(&mut self.input);
                    self.run(cmd);
                    r.request_focus();
                }
            });
    }

    fn health_page(&mut self, ui: &mut Ui, p: &Pal) {
        ui.label(RichText::new("SAĞLIK MONİTÖRÜ").size(20.0).strong().color(p.accent));

        // Oksijen + Radyasyon
        let o2c = level_color(p, self.oxygen, 95.0, 90.0);
        let radc = level_color(p, 100.0 - self.radiation, 50.0, 25.0);
        ui.columns(2, |c| {
            metric(&mut c[0], p, "OKSİJEN SEVİYESİ", self.oxygen, o2c);
            metric(&mut c[1], p, "UZAY RADYASYONU", self.radiation, radc);
        });

        // Kan sayımı
        card(ui, p, "KAN SAYIMI", |ui| {
            blood_row(ui, p, "Basınç", &mut self.pressure, 70.0..=190.0, "mmHg", p.accent);
            blood_row(ui, p, "Nitrojen Doygunluğu", &mut self.nitrogen, 50.0..=100.0, "%", p.accent);
            blood_row(ui, p, "Hemoglobin", &mut self.hemoglobin, 8.0..=20.0, "g/dL", p.accent);
        });

        // Zaman çizelgesi
        card(ui, p, "BİYOMETRİK ZAMAN ÇİZELGESİ – NABIZ (BPM)", |ui| {
            let pts: PlotPoints = self.hr_hist.iter().copied().collect();
            Plot::new("hr_plot")
                .height(150.0)
                .allow_zoom(false)
                .allow_drag(false)
                .allow_scroll(false)
                .allow_boxed_zoom(false)
                .show_axes([false, true])
                .include_y(50.0)
                .include_y(130.0)
                .show(ui, |pu| pu.line(Line::new(pts).color(p.accent).width(2.5)));
        });

        // İzolasyon pivot modeli
        card(ui, p, "İZOLASYON PİVOT MODELİ", |ui| {
            ui.label(RichText::new(format!("Mevcut eksen yönelimi: {:.1}°", self.pivot_deg)).color(p.dim));
            let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 150.0), Sense::hover());
            let c = rect.center();
            let rad = 66.0;
            let painter = ui.painter();
            painter.circle_stroke(c, rad, Stroke::new(2.0, p.edge));
            painter.circle_stroke(c, rad * 0.5, Stroke::new(1.0, p.edge));
            let a = self.pivot_deg.to_radians();
            let tip = Pos2::new(c.x + a.sin() * rad, c.y - a.cos() * rad);
            painter.line_segment([c, tip], Stroke::new(4.0, p.danger));
            painter.circle_filled(c, 6.0, p.accent);
            ui.spacing_mut().slider_width = ui.available_width() - 8.0;
            ui.add(egui::Slider::new(&mut self.pivot_deg, 0.0..=360.0).show_value(false).trailing_fill(true));
            ui.label(RichText::new("Manuel kalibrasyon").size(12.0).color(p.dim));
        });
    }

    fn settings_page(&mut self, ui: &mut Ui, p: &Pal) {
        ui.label(RichText::new("AYARLAR").size(20.0).strong().color(p.accent));
        card(ui, p, "PARLAKLIK", |ui| {
            ui.spacing_mut().slider_width = ui.available_width() - 8.0;
            ui.add(egui::Slider::new(&mut self.brightness, 0.3..=1.5).show_value(false).trailing_fill(true));
            ui.label(RichText::new(format!("%{:.0}", self.brightness * 100.0)).size(24.0).strong());
        });
        card(ui, p, "RENKLER (RGBA)", |ui| {
            ui.horizontal(|ui| {
                ui.color_edit_button_srgba_unmultiplied(&mut self.c_accent);
                ui.label("Vurgu rengi");
            });
            ui.horizontal(|ui| {
                ui.color_edit_button_srgba_unmultiplied(&mut self.c_panel);
                ui.label("Panel rengi (alfa dahil)");
            });
            ui.horizontal(|ui| {
                ui.color_edit_button_srgba_unmultiplied(&mut self.c_bg);
                ui.label("Arka plan");
            });
        });
        if ui.add(egui::Button::new(RichText::new("Varsayılana dön").strong())).clicked() {
            self.brightness = 1.0;
            self.c_bg = [10, 16, 28, 255];
            self.c_panel = [22, 32, 50, 240];
            self.c_accent = [70, 190, 255, 255];
        }
    }
}
