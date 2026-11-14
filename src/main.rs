// AEGIS-BIO – Astronot Kol Tableti (YATAY arayüz)
// Rust + eframe/egui 0.27
use eframe::egui::{
    self, Align, Align2, Color32, FontId, Key, Layout, Pos2, Rect, RichText, Sense, Shape, Stroke,
    Ui, Vec2,
};
use egui_plot::{Line, Plot, PlotPoints};
use std::collections::VecDeque;

const DATASET_PATH: &str = "data/aegis_dataset.txt";

const DEFAULT_DATASET: &str = r#"# AEGIS-BIO veri seti  (anahtar=değer)
# Düzenleyip  SEÇENEKLER > "Veri setini yeniden oku"  ile içeri alın.
planet=MARS
heading=40
hub_dist=340
hub_bearing=85
hub_pressure=101.2
temp_c=-63
atm_pa=610
dust=22
wind_speed=7.5
wind_dir=250
radiation=12.5
bpm=72
spo2=98
pressure_sys=120
pressure_dia=78
nitrogen=78
hemoglobin=14.5
sodium_mmol=139.0
sodium_age_min=135
sleep_hours=6.5
sleep_quality=72
hub_astronauts=Cdr. Emir;Dr. Aylin (Uçuş Hekimi);Müh. Kaan;Müh. Selin
door.ana_kilit=closed
door.yedek_kilit=closed
door.rover_girisi=open
door.sera_gecidi=closed
monitor.astroskin=on
monitor.kol_tableti=on
monitor.giysi_basinc=on
monitor.kabin_co2=on
monitor.dozimetre=on
monitor.uyku=off
monitor.sodyum_yama=on
monitor.hub_hava=on
suit.sol_kol=tear
"#;

// ───────────────────────── Giriş ─────────────────────────
fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 700.0])
            .with_min_inner_size([900.0, 560.0])
            .with_title("AEGIS-BIO – Kol Tableti"),
        ..Default::default()
    };
    eframe::run_native(
        "AEGIS-BIO Kol Tableti",
        options,
        Box::new(|cc| Box::new(App::new(cc))),
    )
}

// ───────────────────────── Tipler ─────────────────────────
#[derive(PartialEq, Clone, Copy)]
enum Page {
    Home,
    Health,
    Hazards,
    Hub,
    Settings,
}

const TABS: [(Page, &str); 5] = [
    (Page::Home, "ANA EKRAN"),
    (Page::Health, "SAĞLIK"),
    (Page::Hazards, "TEHLİKELER"),
    (Page::Hub, "HUB"),
    (Page::Settings, "SEÇENEKLER"),
];

/// Kıyafet bölgesi durumu
#[derive(Clone, Copy, PartialEq)]
enum Suit {
    Ok,
    Tear,
    Crit,
}
impl Suit {
    fn label(self) -> &'static str {
        match self {
            Suit::Ok => "NORMAL",
            Suit::Tear => "YIRTIK",
            Suit::Crit => "KRİTİK",
        }
    }
    fn parse(s: &str) -> Option<Suit> {
        match s {
            "ok" | "normal" => Some(Suit::Ok),
            "tear" | "yirtik" => Some(Suit::Tear),
            "crit" | "kritik" => Some(Suit::Crit),
            _ => None,
        }
    }
}

struct Part {
    id: &'static str,
    name: &'static str,
    circle: bool,
    // 100 x 168 birimlik figür kutusunda konum
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

const PARTS: [Part; 10] = [
    Part { id: "kask", name: "Kask", circle: true, x: 34.0, y: 0.0, w: 32.0, h: 32.0 },
    Part { id: "govde", name: "Gövde", circle: false, x: 32.0, y: 35.0, w: 36.0, h: 56.0 },
    Part { id: "sol_kol", name: "Sol Kol", circle: false, x: 14.0, y: 37.0, w: 16.0, h: 46.0 },
    Part { id: "sag_kol", name: "Sağ Kol", circle: false, x: 70.0, y: 37.0, w: 16.0, h: 46.0 },
    Part { id: "sol_eldiven", name: "Sol Eldiven", circle: false, x: 13.0, y: 86.0, w: 18.0, h: 16.0 },
    Part { id: "sag_eldiven", name: "Sağ Eldiven", circle: false, x: 69.0, y: 86.0, w: 18.0, h: 16.0 },
    Part { id: "sol_bacak", name: "Sol Bacak", circle: false, x: 33.0, y: 94.0, w: 16.0, h: 56.0 },
    Part { id: "sag_bacak", name: "Sağ Bacak", circle: false, x: 51.0, y: 94.0, w: 16.0, h: 56.0 },
    Part { id: "sol_bot", name: "Sol Bot", circle: false, x: 31.0, y: 153.0, w: 19.0, h: 15.0 },
    Part { id: "sag_bot", name: "Sağ Bot", circle: false, x: 50.0, y: 153.0, w: 19.0, h: 15.0 },
];

#[derive(Clone, Copy, PartialEq)]
enum Door {
    Closed,
    Open,
    Cycling,
}
struct DoorRow {
    id: &'static str,
    name: &'static str,
    state: Door,
}
struct Monitor {
    id: &'static str,
    name: &'static str,
    on: bool,
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
    warn: Color32, // turuncu
    danger: Color32, // kırmızı
    bar_bg: Color32,
    suit: Color32,
}

/// RGBA (unmultiplied) renge parlaklık çarpanı uygular.
fn sc(c: [u8; 4], k: f32) -> Color32 {
    let f = |v: u8| ((v as f32) * k).clamp(0.0, 255.0) as u8;
    Color32::from_rgba_unmultiplied(f(c[0]), f(c[1]), f(c[2]), c[3])
}

fn suit_color(p: &Pal, s: Suit) -> Color32 {
    match s {
        Suit::Ok => p.ok,
        Suit::Tear => p.warn,
        Suit::Crit => p.danger,
    }
}

// ───────────────────────── Uygulama durumu ─────────────────────────
struct App {
    page: Page,
    t: f64,

    // Seçenekler
    brightness: f32,
    c_bg: [u8; 4],
    c_panel: [u8; 4],
    c_accent: [u8; 4],
    c_ok: [u8; 4],
    c_warn: [u8; 4],
    c_danger: [u8; 4],
    fahrenheit: bool,
    show_terminal: bool,
    blink: bool,
    ui_scale: f32,
    sync_msg: String,

    // Sağlık
    bpm: f32,
    bpm_base: f32,
    spo2: f32,
    sys: f32,
    dia: f32,
    nitrogen: f32,
    hemoglobin: f32,
    sodium: f32,
    sodium_age_min: f32,
    sleep_hours: f32,
    sleep_quality: f32,

    // Tehlikeler
    radiation: f32,
    planet: String,
    temp_c: f32,
    atm_pa: f32,
    dust: f32,
    wind_speed: f32,
    wind_dir: f32,
    heading: f32,

    // Hub
    hub_dist: f32,
    hub_bearing: f32,
    hub_pressure: f32,
    doors: Vec<DoorRow>,
    hub_astronauts: Vec<String>,
    monitors: Vec<Monitor>,

    // Kıyafet
    suit: [Suit; 10],
    sel: usize,

    // Grafik
    hr_hist: VecDeque<[f64; 2]>,
    last_sample: f64,

    // Terminal
    logs: Vec<String>,
    input: String,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.style_mut(|s| {
            s.spacing.interact_size.y = 40.0;
            s.spacing.item_spacing = Vec2::new(10.0, 8.0);
            s.spacing.button_padding = Vec2::new(14.0, 8.0);
        });
        let mut app = Self {
            page: Page::Home,
            t: 0.0,
            brightness: 1.0,
            c_bg: [10, 16, 28, 255],
            c_panel: [22, 32, 50, 240],
            c_accent: [70, 190, 255, 255],
            c_ok: [72, 212, 142, 255],
            c_warn: [255, 140, 30, 255],
            c_danger: [255, 62, 62, 255],
            fahrenheit: false,
            show_terminal: true,
            blink: true,
            ui_scale: 1.0,
            sync_msg: "Henüz senkronize edilmedi.".into(),
            bpm: 72.0,
            bpm_base: 72.0,
            spo2: 98.0,
            sys: 120.0,
            dia: 78.0,
            nitrogen: 78.0,
            hemoglobin: 14.5,
            sodium: 139.0,
            sodium_age_min: 135.0,
            sleep_hours: 6.5,
            sleep_quality: 72.0,
            radiation: 12.5,
            planet: "MARS".into(),
            temp_c: -63.0,
            atm_pa: 610.0,
            dust: 22.0,
            wind_speed: 7.5,
            wind_dir: 250.0,
            heading: 40.0,
            hub_dist: 340.0,
            hub_bearing: 85.0,
            hub_pressure: 101.2,
            doors: vec![
                DoorRow { id: "ana_kilit", name: "Ana Hava Kilidi", state: Door::Closed },
                DoorRow { id: "yedek_kilit", name: "Yedek Hava Kilidi", state: Door::Closed },
                DoorRow { id: "rover_girisi", name: "Rover Giriş Kapısı", state: Door::Open },
                DoorRow { id: "sera_gecidi", name: "Sera Geçidi", state: Door::Closed },
            ],
            hub_astronauts: vec![],
            monitors: vec![
                Monitor { id: "astroskin", name: "Astroskin Göğüs Sensörü", on: true },
                Monitor { id: "kol_tableti", name: "Kol Tableti", on: true },
                Monitor { id: "giysi_basinc", name: "Kıyafet Basınç Ağı", on: true },
                Monitor { id: "kabin_co2", name: "Kabin CO2 Monitörü", on: true },
                Monitor { id: "dozimetre", name: "Radyasyon Dozimetresi", on: true },
                Monitor { id: "uyku", name: "Uyku İzleyici", on: false },
                Monitor { id: "sodyum_yama", name: "Sodyum / Ter Yaması", on: true },
                Monitor { id: "hub_hava", name: "Hub Hava Monitörü", on: true },
            ],
            suit: [Suit::Ok; 10],
            sel: 2,
            hr_hist: VecDeque::new(),
            last_sample: 0.0,
            logs: vec![
                "[SİSTEM] AegisOS kol tableti hazır. Komutlar için 'help'.".into(),
            ],
            input: String::new(),
        };
        app.sync_dataset();
        app
    }

    fn pal(&self) -> Pal {
        let k = self.brightness;
        Pal {
            bg: sc(self.c_bg, k),
            panel: sc(self.c_panel, k),
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
            ok: sc(self.c_ok, k),
            warn: sc(self.c_warn, k),
            danger: sc(self.c_danger, k),
            bar_bg: sc([12, 18, 30, 255], k),
            suit: sc([176, 194, 216, 255], k),
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
        ctx.set_visuals(v);
    }

    fn log(&mut self, s: impl Into<String>) {
        self.logs.push(s.into());
        if self.logs.len() > 200 {
            self.logs.remove(0);
        }
    }

    fn temp_str(&self, c: f32) -> String {
        if self.fahrenheit {
            format!("{:.0} °F", c * 9.0 / 5.0 + 32.0)
        } else {
            format!("{c:.0} °C")
        }
    }

    // ── Kıyafet özeti ──
    fn suit_summary(&self, p: &Pal) -> (String, Color32) {
        let names = |s: Suit| -> Vec<&str> {
            PARTS.iter().enumerate().filter(|(i, _)| self.suit[*i] == s).map(|(_, pt)| pt.name).collect()
        };
        let crit = names(Suit::Crit);
        let tear = names(Suit::Tear);
        if !crit.is_empty() {
            (format!("KRİTİK: {}", crit.join(", ")), p.danger)
        } else if !tear.is_empty() {
            (format!("YIRTIK: {}", tear.join(", ")), p.warn)
        } else {
            ("KIYAFET BÜTÜNLÜĞÜ NORMAL".into(), p.ok)
        }
    }

    // ── Veri seti okuma ──
    fn apply_kv(&mut self, k: &str, v: &str) -> bool {
        fn set(dst: &mut f32, v: Option<f32>) -> bool {
            if let Some(x) = v {
                *dst = x;
                true
            } else {
                false
            }
        }
        if let Some(id) = k.strip_prefix("door.") {
            let st = match v {
                "open" => Door::Open,
                "closed" => Door::Closed,
                "cycling" => Door::Cycling,
                _ => return false,
            };
            return self.doors.iter_mut().find(|d| d.id == id).map(|d| d.state = st).is_some();
        }
        if let Some(id) = k.strip_prefix("monitor.") {
            let on = match v {
                "on" => true,
                "off" => false,
                _ => return false,
            };
            return self.monitors.iter_mut().find(|m| m.id == id).map(|m| m.on = on).is_some();
        }
        if let Some(id) = k.strip_prefix("suit.") {
            let st = match Suit::parse(v) {
                Some(s) => s,
                None => return false,
            };
            return match PARTS.iter().position(|p| p.id == id) {
                Some(i) => {
                    self.suit[i] = st;
                    true
                }
                None => false,
            };
        }
        let f = v.parse::<f32>().ok();
        match k {
            "planet" => {
                self.planet = v.to_uppercase();
                true
            }
            "hub_astronauts" => {
                self.hub_astronauts = v.split(';').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                true
            }
            "bpm" => {
                let ok = set(&mut self.bpm_base, f);
                if ok {
                    self.bpm = self.bpm_base;
                }
                ok
            }
            "spo2" => set(&mut self.spo2, f),
            "pressure_sys" => set(&mut self.sys, f),
            "pressure_dia" => set(&mut self.dia, f),
            "nitrogen" => set(&mut self.nitrogen, f),
            "hemoglobin" => set(&mut self.hemoglobin, f),
            "sodium_mmol" => set(&mut self.sodium, f),
            "sodium_age_min" => set(&mut self.sodium_age_min, f),
            "sleep_hours" => set(&mut self.sleep_hours, f),
            "sleep_quality" => set(&mut self.sleep_quality, f),
            "radiation" => set(&mut self.radiation, f),
            "temp_c" => set(&mut self.temp_c, f),
            "atm_pa" => set(&mut self.atm_pa, f),
            "dust" => set(&mut self.dust, f),
            "wind_speed" => set(&mut self.wind_speed, f),
            "wind_dir" => set(&mut self.wind_dir, f),
            "heading" => set(&mut self.heading, f),
            "hub_dist" => set(&mut self.hub_dist, f),
            "hub_bearing" => set(&mut self.hub_bearing, f),
            "hub_pressure" => set(&mut self.hub_pressure, f),
            _ => false,
        }
    }

    fn sync_dataset(&mut self) {
        let path = std::path::Path::new(DATASET_PATH);
        if !path.exists() {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            match std::fs::write(path, DEFAULT_DATASET) {
                Ok(_) => self.log(format!("[SYNC] {DATASET_PATH} yoktu, örnek veri seti oluşturuldu.")),
                Err(e) => {
                    self.sync_msg = format!("Dosya oluşturulamadı: {e}");
                    self.log(format!("[HATA] {}", self.sync_msg));
                    return;
                }
            }
        }
        match std::fs::read_to_string(path) {
            Ok(txt) => {
                let (mut ok, mut bad) = (0, 0);
                for line in txt.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    match line.split_once('=') {
                        Some((k, v)) if self.apply_kv(k.trim(), v.trim()) => ok += 1,
                        _ => bad += 1,
                    }
                }
                self.sync_msg = format!("{ok} değer okundu, {bad} satır atlandı.");
                self.log(format!("[SYNC] {}", self.sync_msg));
            }
            Err(e) => {
                self.sync_msg = format!("Okunamadı: {e}");
                self.log(format!("[HATA] {}", self.sync_msg));
            }
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
                    "home | health | hazards | hub | settings   sekme aç",
                    "status                anlık değerler",
                    "sync                  veri setini yeniden oku",
                    "suit                  kıyafet bölgelerini listele",
                    "suit <bolge> <ok|tear|crit>   örn: suit sol_kol tear",
                    "bpm <n> | o2 <n> | rad <n>    değer ayarla",
                    "clear                 terminali temizle",
                ] {
                    self.log(l);
                }
            }
            "status" => {
                let s = format!(
                    "BPM {:.0} | SpO2 %{:.0} | RAD %{:.1} | BP {:.0}/{:.0} | Na {:.1} | Hub {:.0} m",
                    self.bpm, self.spo2, self.radiation, self.sys, self.dia, self.sodium, self.hub_dist
                );
                self.log(s);
            }
            "home" => self.page = Page::Home,
            "health" => self.page = Page::Health,
            "hazards" => self.page = Page::Hazards,
            "hub" => self.page = Page::Hub,
            "settings" => self.page = Page::Settings,
            "sync" => self.sync_dataset(),
            "suit" => {
                if parts.len() >= 3 {
                    match (PARTS.iter().position(|p| p.id == parts[1]), Suit::parse(parts[2])) {
                        (Some(i), Some(st)) => {
                            self.suit[i] = st;
                            self.sel = i;
                            self.log(format!("[OK] {} → {}", PARTS[i].name, st.label()));
                        }
                        _ => self.log("[HATA] kullanım: suit sol_kol tear"),
                    }
                } else {
                    for (i, pt) in PARTS.iter().enumerate() {
                        let l = format!("{:<12} {}", pt.id, self.suit[i].label());
                        self.log(l);
                    }
                }
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
                    self.spo2 = n.clamp(0.0, 100.0);
                    self.log(format!("[OK] SpO2 %{:.0}", self.spo2));
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
        self.t = t;
        let target = self.bpm_base + 4.0 * (t * 0.9).sin() as f32 + 2.0 * (t * 2.3).sin() as f32;
        self.bpm += (target - self.bpm) * 0.05;
        if t - self.last_sample > 0.1 {
            self.last_sample = t;
            self.hr_hist.push_back([t, self.bpm as f64]);
            while self.hr_hist.front().map_or(false, |p| p[0] < t - 20.0) {
                self.hr_hist.pop_front();
            }
        }
    }
}

// ───────────────────────── Değer → renk ─────────────────────────
fn lvl_low(p: &Pal, v: f32, warn_below: f32, danger_below: f32) -> Color32 {
    if v < danger_below {
        p.danger
    } else if v < warn_below {
        p.warn
    } else {
        p.ok
    }
}
fn lvl_band(p: &Pal, v: f32, ok_lo: f32, ok_hi: f32, bad_lo: f32, bad_hi: f32) -> Color32 {
    if v < bad_lo || v > bad_hi {
        p.danger
    } else if v < ok_lo || v > ok_hi {
        p.warn
    } else {
        p.ok
    }
}
fn cardinal(deg: f32) -> &'static str {
    ["K", "KD", "D", "GD", "G", "GB", "B", "KB"][((deg.rem_euclid(360.0) + 22.5) / 45.0) as usize % 8]
}
fn rel_word(rel: f32) -> &'static str {
    ["ÖN", "SAĞ-ÖN", "SAĞ", "SAĞ-ARKA", "ARKA", "SOL-ARKA", "SOL", "SOL-ÖN"]
        [((rel.rem_euclid(360.0) + 22.5) / 45.0) as usize % 8]
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
        mesh.colored_vertex(Pos2::new(c.x + x * size / 34.0, c.y - (y + 2.5) * size / 34.0), color);
    }
    for i in 0..n {
        mesh.add_triangle(0, 1 + i as u32, 1 + ((i + 1) % n) as u32);
    }
    painter.add(Shape::mesh(mesh));
}

fn heart_beat(ui: &mut Ui, p: &Pal, size: f32, bpm: f32, t: f64) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size * 1.35), Sense::hover());
    let phase = ((t * bpm as f64 / 60.0) % 1.0) as f32;
    let pulse = (1.0 - phase * 3.0).max(0.0).powi(2);
    heart(ui.painter(), rect.center(), size * (1.0 + 0.2 * pulse), p.danger);
}

fn card(ui: &mut Ui, p: &Pal, title: &str, min_h: f32, add: impl FnOnce(&mut Ui)) -> egui::Response {
    egui::Frame::none()
        .fill(p.panel)
        .stroke(Stroke::new(1.0, p.edge))
        .rounding(14.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if min_h > 0.0 {
                ui.set_min_height((min_h - 24.0).max(0.0));
            }
            ui.label(RichText::new(title).size(12.0).strong().color(p.dim));
            ui.add_space(4.0);
            add(ui);
        })
        .response
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

/// Dikey bar + sağında büyük değer yazısı
fn metric(ui: &mut Ui, p: &Pal, title: &str, frac: f32, text: String, color: Color32, sub: &str) {
    card(ui, p, title, 0.0, |ui| {
        ui.horizontal(|ui| {
            vbar(ui, p, frac, color, Vec2::new(40.0, 140.0));
            ui.vertical(|ui| {
                ui.add_space(34.0);
                ui.label(RichText::new(text).size(38.0).strong().color(color));
                ui.label(RichText::new(sub).size(13.0).color(p.dim));
            });
        });
    });
}

/// Yatay aralık göstergesi (yeşil = normal bölge)
fn range_bar(ui: &mut Ui, p: &Pal, val: f32, min: f32, max: f32, ok_lo: f32, ok_hi: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 22.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 6.0, p.bar_bg);
    let x = |v: f32| rect.left() + rect.width() * ((v - min) / (max - min)).clamp(0.0, 1.0);
    let zone = Rect::from_min_max(Pos2::new(x(ok_lo), rect.top()), Pos2::new(x(ok_hi), rect.bottom()));
    painter.rect_filled(zone, 4.0, p.ok.gamma_multiply(0.45));
    painter.rect_stroke(rect, 6.0, Stroke::new(1.0, p.edge));
    let mx = x(val);
    painter.line_segment([Pos2::new(mx, rect.top() - 2.0), Pos2::new(mx, rect.bottom() + 2.0)], Stroke::new(4.0, p.text));
}

fn kv(ui: &mut Ui, p: &Pal, k: &str, v: String, col: Color32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(k).color(p.dim));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(v).size(20.0).strong().color(col));
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

/// Pusula kadranı. needle_deg: ibre açısı (üst = 0°, saat yönü)
fn compass(ui: &mut Ui, p: &Pal, size: f32, labels: [&str; 4], needle_deg: f32, needle_col: Color32, marker: Option<(f32, Color32)>) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let c = rect.center();
    let r = size * 0.5 - 4.0;
    let painter = ui.painter();
    painter.circle_filled(c, r, p.bar_bg);
    painter.circle_stroke(c, r, Stroke::new(2.0, p.edge));
    for i in 0..36 {
        let a = (i as f32 * 10.0).to_radians();
        let d = Vec2::new(a.sin(), -a.cos());
        let len = if i % 3 == 0 { 10.0 } else { 5.0 };
        painter.line_segment([c + d * (r - len), c + d * r], Stroke::new(1.0, p.dim));
    }
    for (i, l) in labels.iter().enumerate() {
        let a = (i as f32 * 90.0).to_radians();
        let d = Vec2::new(a.sin(), -a.cos());
        painter.text(c + d * (r - 24.0), Align2::CENTER_CENTER, *l, FontId::proportional(13.0), if i == 0 { p.accent } else { p.dim });
    }
    if let Some((deg, col)) = marker {
        let a = deg.to_radians();
        let d = Vec2::new(a.sin(), -a.cos());
        painter.circle_filled(c + d * (r - 9.0), 7.0, col);
    }
    let a = needle_deg.to_radians();
    let d = Vec2::new(a.sin(), -a.cos());
    let n = Vec2::new(-d.y, d.x);
    let tip = c + d * (r * 0.55);
    let base = c + d * (r * 0.1);
    painter.add(Shape::convex_polygon(vec![tip, base + n * 7.0, base - n * 7.0], needle_col, Stroke::NONE));
    painter.line_segment([c, c - d * (r * 0.22)], Stroke::new(3.0, p.dim));
    painter.circle_filled(c, 5.0, p.text);
}

// ───────────────────────── Ana döngü ─────────────────────────
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let t = ctx.input(|i| i.time);
        self.simulate(t);
        let p = self.pal();
        self.apply_visuals(ctx, &p);
        if (ctx.zoom_factor() - self.ui_scale).abs() > 0.001 {
            ctx.set_zoom_factor(self.ui_scale);
        }
        ctx.request_repaint();

        // ── Üst çubuk ──
        egui::TopBottomPanel::top("header")
            .exact_height(52.0)
            .frame(egui::Frame::none().fill(p.panel).inner_margin(egui::Margin::symmetric(14.0, 6.0)))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new("AEGIS-BIO").size(20.0).strong().color(p.accent));
                    ui.label(RichText::new(format!("{}  •  EVA_SPACEWALK", self.planet)).size(13.0).color(p.dim));
                    let (txt, col) = self.suit_summary(&p);
                    if col != p.ok {
                        let k = if self.blink && col == p.danger { 0.6 + 0.4 * (t * 6.0).sin().abs() as f32 } else { 1.0 };
                        let [r, g, b, _] = col.to_srgba_unmultiplied();
                        let c2 = Color32::from_rgb((r as f32 * k) as u8, (g as f32 * k) as u8, (b as f32 * k) as u8);
                        ui.label(RichText::new(format!(" {txt} ")).strong().color(Color32::BLACK).background_color(c2));
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let s = t as u64;
                        ui.label(RichText::new(format!("MET {:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)).size(14.0).color(p.dim));
                        ui.label(RichText::new(format!("{:.0} BPM", self.bpm)).size(16.0).strong().color(p.danger));
                    });
                });
            });

        // ── Alt: terminal ──
        if self.show_terminal {
            egui::TopBottomPanel::bottom("terminal")
                .exact_height(170.0)
                .frame(egui::Frame::none().fill(p.bg).inner_margin(egui::Margin::same(10.0)))
                .show(ctx, |ui| self.terminal(ui, &p));
        }

        // ── Sol: sekmeler ──
        egui::SidePanel::left("nav")
            .exact_width(160.0)
            .resizable(false)
            .frame(egui::Frame::none().fill(p.panel).inner_margin(egui::Margin::same(10.0)))
            .show(ctx, |ui| {
                for (pg, label) in TABS {
                    let active = self.page == pg;
                    let btn = egui::Button::new(
                        RichText::new(label).size(16.0).strong().color(if active { Color32::BLACK } else { p.text }),
                    )
                    .fill(if active { p.accent } else { p.panel_hi })
                    .min_size(Vec2::new(ui.available_width(), 64.0));
                    if ui.add(btn).clicked() {
                        self.page = pg;
                    }
                }
            });

        // ── İçerik ──
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(p.bg).inner_margin(egui::Margin::same(12.0)))
            .show(ctx, |ui| match self.page {
                Page::Home => self.home(ui, &p, t),
                _ => {
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match self.page {
                        Page::Health => self.health_page(ui, &p, t),
                        Page::Hazards => self.hazards_page(ui, &p),
                        Page::Hub => self.hub_page(ui, &p),
                        Page::Settings => self.settings_page(ui, &p),
                        Page::Home => {}
                    });
                }
            });
    }
}

// ───────────────────────── Sayfalar ─────────────────────────
impl App {
    fn terminal(&mut self, ui: &mut Ui, p: &Pal) {
        egui::Frame::none()
            .fill(p.bar_bg)
            .stroke(Stroke::new(1.0, p.accent))
            .rounding(12.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(">_ TERMİNAL (G/Ç)").size(12.0).strong().color(p.accent));
                let h = (ui.available_height() - 46.0).max(30.0);
                egui::ScrollArea::vertical().max_height(h).auto_shrink([false, false]).stick_to_bottom(true).show(ui, |ui| {
                    for l in &self.logs {
                        let col = if l.starts_with("[HATA]") {
                            p.danger
                        } else if l.starts_with('>') {
                            p.accent
                        } else if l.starts_with("[SYNC]") {
                            p.ok
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

    // ───── ANA EKRAN: kalp atışı, hub pusulası, space suit izolasyonu ─────
    fn home(&mut self, ui: &mut Ui, p: &Pal, t: f64) {
        let hh = ui.available_height();
        ui.columns(3, |c| {
            // Kolon 1: kalp + hub pusulası
            let h1 = hh * 0.40;
            let r = card(&mut c[0], p, "KALP ATIŞI  (sağlık için dokun)", h1, |ui| {
                ui.horizontal(|ui| {
                    heart_beat(ui, p, 62.0, self.bpm, t);
                    ui.vertical(|ui| {
                        let col = lvl_band(p, self.bpm, 50.0, 100.0, 40.0, 130.0);
                        ui.label(RichText::new(format!("{:.0}", self.bpm)).size(58.0).strong().color(col));
                        ui.label(RichText::new("BPM").size(16.0).color(p.dim));
                    });
                });
            });
            let id = c[0].id().with("heart_click");
            if c[0].interact(r.rect, id, Sense::click()).clicked() {
                self.page = Page::Health;
            }

            let rel = (self.hub_bearing - self.heading).rem_euclid(360.0);
            let dial = ((hh - h1 - 8.0) - 24.0 - 20.0 - 70.0).clamp(90.0, 170.0);
            card(&mut c[0], p, "MERKEZ (HUB) PUSULASI", hh - h1 - 8.0, |ui| {
                ui.vertical_centered(|ui| {
                    compass(ui, p, dial, ["ÖN", "SAĞ", "ARKA", "SOL"], rel, p.accent, None);
                });
                kv(ui, p, "Uzaklık", format!("{:.0} m", self.hub_dist), p.text);
                kv(ui, p, "Yön", format!("{:.0}° • {}", rel, rel_word(rel)), p.accent);
            });

            // Kolon 2: space suit figürü
            card(&mut c[1], p, "SPACE SUIT İZOLASYONU", hh, |ui| {
                let avail = Vec2::new(ui.available_width(), (hh - 110.0).max(120.0));
                self.suit_figure(ui, p, avail, t);
                ui.horizontal(|ui| {
                    for s in [Suit::Ok, Suit::Tear, Suit::Crit] {
                        let (rc, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                        ui.painter().rect_filled(rc, 3.0, suit_color(p, s));
                        ui.label(RichText::new(s.label()).size(12.0).color(p.dim));
                    }
                });
            });

            // Kolon 3: bölge durumu
            card(&mut c[2], p, "BÖLGE DURUMU", hh, |ui| {
                let (txt, col) = self.suit_summary(p);
                ui.label(RichText::new(txt).size(17.0).strong().color(col));
                ui.add_space(2.0);
                egui::ScrollArea::vertical().max_height((hh - 190.0).max(80.0)).auto_shrink([false, false]).show(ui, |ui| {
                    for i in 0..PARTS.len() {
                        self.part_row(ui, p, i);
                    }
                });
                ui.separator();
                let sel = self.sel;
                let cur = self.suit[sel];
                let tip = match cur {
                    Suit::Ok => "Bölge sağlam.",
                    Suit::Tear => "Yama kiti uygula, basınç kaybını izle.",
                    Suit::Crit => "ACİL: hub'a dön, hava kilidini hazırla!",
                };
                ui.label(RichText::new(format!("{}: {}", PARTS[sel].name, tip)).size(13.0).color(suit_color(p, cur)));
                ui.horizontal(|ui| {
                    for (lab, st) in [("NORMAL", Suit::Ok), ("YIRTIK", Suit::Tear), ("KRİTİK", Suit::Crit)] {
                        let b = egui::Button::new(RichText::new(lab).strong().color(Color32::BLACK)).fill(suit_color(p, st));
                        if ui.add(b).clicked() {
                            self.suit[sel] = st;
                            self.log(format!("[SUIT] {} → {}", PARTS[sel].name, st.label()));
                        }
                    }
                });
            });
        });
    }

    fn part_row(&mut self, ui: &mut Ui, p: &Pal, i: usize) {
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.0), Sense::click());
        let st = self.suit[i];
        let col = suit_color(p, st);
        let painter = ui.painter();
        if self.sel == i {
            painter.rect_filled(rect, 8.0, p.panel_hi);
            painter.rect_stroke(rect, 8.0, Stroke::new(1.5, p.accent));
        } else if resp.hovered() {
            painter.rect_filled(rect, 8.0, p.panel_hi.gamma_multiply(0.6));
        }
        painter.circle_filled(Pos2::new(rect.left() + 14.0, rect.center().y), 6.0, col);
        painter.text(Pos2::new(rect.left() + 30.0, rect.center().y), Align2::LEFT_CENTER, PARTS[i].name, FontId::proportional(15.0), p.text);
        painter.text(Pos2::new(rect.right() - 10.0, rect.center().y), Align2::RIGHT_CENTER, st.label(), FontId::proportional(14.0), col);
        if resp.clicked() {
            self.sel = i;
        }
    }

    fn suit_figure(&mut self, ui: &mut Ui, p: &Pal, avail: Vec2, t: f64) {
        let (rect, _) = ui.allocate_exact_size(avail, Sense::hover());
        let s = (rect.width() / 100.0).min(rect.height() / 168.0);
        let origin = Pos2::new(rect.center().x - 50.0 * s, rect.center().y - 84.0 * s);
        let painter = ui.painter().clone();
        let pulse = if self.blink { 0.6 + 0.4 * (t * 6.0).sin().abs() as f32 } else { 1.0 };

        for (i, part) in PARTS.iter().enumerate() {
            let r = Rect::from_min_size(
                Pos2::new(origin.x + part.x * s, origin.y + part.y * s),
                Vec2::new(part.w * s, part.h * s),
            );
            let resp = ui.interact(r, ui.id().with(("suit_part", i)), Sense::click());
            let st = self.suit[i];
            let fill = match st {
                Suit::Ok => p.suit,
                Suit::Tear => p.warn,
                Suit::Crit => {
                    let [cr, cg, cb, _] = p.danger.to_srgba_unmultiplied();
                    Color32::from_rgb((cr as f32 * pulse) as u8, (cg as f32 * pulse) as u8, (cb as f32 * pulse) as u8)
                }
            };
            let edge = if resp.hovered() { p.text } else { p.edge };
            if part.circle {
                painter.circle_filled(r.center(), r.width() / 2.0, fill);
                painter.circle_stroke(r.center(), r.width() / 2.0, Stroke::new(2.0, edge));
                // vizör
                let v = Rect::from_center_size(r.center() + Vec2::new(0.0, 1.5 * s), Vec2::new(r.width() * 0.7, r.height() * 0.42));
                painter.rect_filled(v, 8.0 * s.min(1.5), p.bar_bg);
            } else {
                painter.rect_filled(r, 6.0 * s.min(1.5), fill);
                painter.rect_stroke(r, 6.0 * s.min(1.5), Stroke::new(2.0, edge));
            }
            // yırtık işareti (zikzak)
            if st != Suit::Ok {
                let pts: Vec<Pos2> = (0..6)
                    .map(|k| {
                        let fx = r.left() + r.width() * (0.12 + 0.76 * k as f32 / 5.0);
                        let fy = r.center().y + if k % 2 == 0 { -1.0 } else { 1.0 } * r.height().min(r.width()) * 0.16;
                        Pos2::new(fx, fy)
                    })
                    .collect();
                painter.add(Shape::line(pts, Stroke::new(2.5, p.bar_bg)));
            }
            if self.sel == i {
                painter.rect_stroke(r.expand(4.0), 8.0, Stroke::new(2.5, p.accent));
            }
            if resp.clicked() {
                self.sel = i;
            }
            resp.on_hover_text(part.name);
        }
    }

    // ───── SAĞLIK ─────
    fn health_page(&mut self, ui: &mut Ui, p: &Pal, t: f64) {
        ui.columns(3, |c| {
            // Kolon 1: oksijen + nabız
            let spo2c = lvl_low(p, self.spo2, 95.0, 90.0);
            metric(&mut c[0], p, "OKSİJEN ORANI (SpO2)", self.spo2 / 100.0, format!("%{:.0}", self.spo2), spo2c, "kan doygunluğu");
            card(&mut c[0], p, "NABIZ", 0.0, |ui| {
                ui.horizontal(|ui| {
                    heart_beat(ui, p, 44.0, self.bpm, t);
                    ui.vertical(|ui| {
                        let col = lvl_band(p, self.bpm, 50.0, 100.0, 40.0, 130.0);
                        ui.label(RichText::new(format!("{:.0} BPM", self.bpm)).size(34.0).strong().color(col));
                    });
                });
            });

            // Kolon 2: kan sayımı + grafik
            card(&mut c[1], p, "KAN BASINCI & KAN SAYIMI", 0.0, |ui| {
                let bc = lvl_band(p, self.sys, 95.0, 135.0, 85.0, 160.0);
                blood_row(ui, p, "Sistolik", &mut self.sys, 70.0..=190.0, "mmHg", bc);
                blood_row(ui, p, "Diyastolik", &mut self.dia, 40.0..=120.0, "mmHg", p.accent);
                blood_row(ui, p, "Nitrojen Doygunluğu", &mut self.nitrogen, 50.0..=100.0, "%", p.accent);
                blood_row(ui, p, "Hemoglobin", &mut self.hemoglobin, 8.0..=20.0, "g/dL", p.accent);
            });
            card(&mut c[1], p, "NABIZ ZAMAN ÇİZELGESİ (BPM)", 0.0, |ui| {
                let pts = PlotPoints::from(self.hr_hist.iter().copied().collect::<Vec<[f64; 2]>>());
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

            // Kolon 3: sodyum + uyku
            let na = self.sodium;
            let nac = lvl_band(p, na, 135.0, 145.0, 130.0, 150.0);
            let age = self.sodium_age_min as u32;
            card(&mut c[2], p, "SODYUM (SON ÖLÇÜM)", 0.0, |ui| {
                ui.label(RichText::new(format!("{na:.1} mmol/L")).size(38.0).strong().color(nac));
                let state = if na < 135.0 { "DÜŞÜK" } else if na > 145.0 { "YÜKSEK" } else { "NORMAL" };
                ui.label(RichText::new(state).size(15.0).strong().color(nac));
                range_bar(ui, p, na, 125.0, 155.0, 135.0, 145.0);
                ui.label(RichText::new(format!("Son ölçüm: {} sa {} dk önce  •  normal 135–145", age / 60, age % 60)).size(12.0).color(p.dim));
            });
            let sl = self.sleep_hours;
            let slc = lvl_low(p, sl, 7.0, 5.0);
            let sq = self.sleep_quality;
            card(&mut c[2], p, "UYKU DURUMU", 0.0, |ui| {
                ui.label(RichText::new(format!("{sl:.1} saat")).size(38.0).strong().color(slc));
                let state = if sl >= 7.0 { "YETERLİ" } else if sl >= 5.0 { "YETERSİZ" } else { "KRİTİK" };
                ui.label(RichText::new(state).size(15.0).strong().color(slc));
                range_bar(ui, p, sl, 0.0, 10.0, 7.0, 9.0);
                ui.add_space(4.0);
                kv(ui, p, "Uyku kalitesi", format!("%{sq:.0}"), lvl_low(p, sq, 60.0, 40.0));
            });
        });
    }

    // ───── TEHLİKELER ─────
    fn hazards_page(&mut self, ui: &mut Ui, p: &Pal) {
        let t = self.t;
        let wind = self.wind_speed + 0.8 * (t * 0.7).sin() as f32;
        ui.columns(3, |c| {
            // Radyasyon + hava durumu
            let rc = if self.radiation >= 75.0 { p.danger } else if self.radiation >= 50.0 { p.warn } else { p.ok };
            metric(&mut c[0], p, "UZAY RADYASYONU", self.radiation / 100.0, format!("%{:.1}", self.radiation), rc, "günlük doz limiti");
            let temp = self.temp_str(self.temp_c);
            card(&mut c[0], p, "GEZEGEN HAVA DURUMU", 0.0, |ui| {
                kv(ui, p, "Gezegen", self.planet.clone(), p.accent);
                kv(ui, p, "Sıcaklık", temp, p.text);
                kv(ui, p, "Atmosfer basıncı", format!("{:.0} Pa", self.atm_pa), p.text);
                let dc = if self.dust >= 70.0 { p.danger } else if self.dust >= 40.0 { p.warn } else { p.ok };
                kv(ui, p, "Toz fırtınası riski", format!("%{:.0}", self.dust), dc);
                range_bar(ui, p, self.dust, 0.0, 100.0, 0.0, 40.0);
            });

            // Rüzgar
            card(&mut c[1], p, "RÜZGAR YÖNÜ & HIZI", 0.0, |ui| {
                ui.vertical_centered(|ui| {
                    // ok, rüzgarın ESTİĞİ yönü gösterir
                    compass(ui, p, 190.0, ["K", "D", "G", "B"], self.wind_dir + 180.0, p.accent, None);
                });
                let wc = if wind >= 25.0 { p.danger } else if wind >= 15.0 { p.warn } else { p.ok };
                kv(ui, p, "Hız", format!("{wind:.1} m/s"), wc);
                kv(ui, p, "Geldiği yön", format!("{:.0}° • {}", self.wind_dir, cardinal(self.wind_dir)), p.text);
            });

            // Gezegene göre pusula
            card(&mut c[2], p, "GEZEGEN PUSULASI", 0.0, |ui| {
                ui.vertical_centered(|ui| {
                    compass(ui, p, 190.0, ["K", "D", "G", "B"], self.heading, p.accent, Some((self.hub_bearing, p.ok)));
                });
                kv(ui, p, "Yönelimin", format!("{:.0}° • {}", self.heading, cardinal(self.heading)), p.accent);
                kv(ui, p, "Hub yönü (yeşil nokta)", format!("{:.0}° • {}", self.hub_bearing, cardinal(self.hub_bearing)), p.ok);
            });
        });
    }

    // ───── HUB ─────
    fn hub_page(&mut self, ui: &mut Ui, p: &Pal) {
        ui.columns(3, |c| {
            let hp = self.hub_pressure;
            let hc = lvl_band(p, hp, 95.0, 105.0, 90.0, 110.0);
            metric(&mut c[0], p, "MERKEZ HAVA BASINCI", hp / 120.0, format!("{hp:.1}"), hc, "kPa");
            card(&mut c[0], p, &format!("MERKEZDEKİ ASTRONOTLAR ({})", self.hub_astronauts.len()), 0.0, |ui| {
                if self.hub_astronauts.is_empty() {
                    ui.label(RichText::new("Hub'da kimse yok.").color(p.dim));
                }
                for a in &self.hub_astronauts {
                    ui.horizontal(|ui| {
                        let (rc, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
                        ui.painter().circle_filled(rc.center(), 6.0, p.ok);
                        ui.label(RichText::new(a).size(16.0));
                    });
                }
            });

            card(&mut c[1], p, "MERKEZ HAVA KİLİDİ (KAPILAR)", 0.0, |ui| {
                for d in self.doors.iter_mut() {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(d.name).size(16.0));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let (lab, col) = match d.state {
                                Door::Closed => ("KAPALI", p.ok),
                                Door::Open => ("AÇIK", p.warn),
                                Door::Cycling => ("DÖNGÜDE", p.accent),
                            };
                            let b = egui::Button::new(RichText::new(lab).strong().color(Color32::BLACK))
                                .fill(col)
                                .min_size(Vec2::new(100.0, 36.0));
                            if ui.add(b).clicked() {
                                d.state = match d.state {
                                    Door::Closed => Door::Cycling,
                                    Door::Cycling => Door::Open,
                                    Door::Open => Door::Closed,
                                };
                            }
                        });
                    });
                }
                ui.label(RichText::new("Dokun: Kapalı → Döngüde → Açık").size(12.0).color(p.dim));
            });

            let active = self.monitors.iter().filter(|m| m.on).count();
            let total = self.monitors.len();
            card(&mut c[2], p, &format!("AKTİF BİO MONİTÖRLER ({active}/{total})"), 0.0, |ui| {
                for m in self.monitors.iter_mut() {
                    ui.horizontal(|ui| {
                        let (rc, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
                        ui.painter().circle_filled(rc.center(), 6.0, if m.on { p.ok } else { p.dim });
                        ui.label(RichText::new(m.name).size(15.0));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let b = egui::Button::new(RichText::new(if m.on { "AKTİF" } else { "KAPALI" }).strong().color(if m.on { Color32::BLACK } else { p.text }))
                                .fill(if m.on { p.ok } else { p.panel_hi })
                                .min_size(Vec2::new(80.0, 32.0));
                            if ui.add(b).clicked() {
                                m.on = !m.on;
                            }
                        });
                    });
                }
            });
        });
    }

    // ───── SEÇENEKLER ─────
    fn settings_page(&mut self, ui: &mut Ui, p: &Pal) {
        ui.columns(2, |c| {
            card(&mut c[0], p, "PARLAKLIK", 0.0, |ui| {
                ui.spacing_mut().slider_width = ui.available_width() - 8.0;
                ui.add(egui::Slider::new(&mut self.brightness, 0.3..=1.5).show_value(false).trailing_fill(true));
                ui.label(RichText::new(format!("%{:.0}", self.brightness * 100.0)).size(28.0).strong());
            });
            card(&mut c[0], p, "RENKLER (RGBA)", 0.0, |ui| {
                for (lab, col) in [
                    ("Vurgu rengi", &mut self.c_accent),
                    ("Panel rengi (alfa dahil)", &mut self.c_panel),
                    ("Arka plan", &mut self.c_bg),
                    ("Normal / sağlam", &mut self.c_ok),
                    ("Yırtık / uyarı (turuncu)", &mut self.c_warn),
                    ("Kritik / tehlike (kırmızı)", &mut self.c_danger),
                ] {
                    ui.horizontal(|ui| {
                        ui.color_edit_button_srgba_unmultiplied(col);
                        ui.label(lab);
                    });
                }
                if ui.button("Varsayılan renkler").clicked() {
                    self.brightness = 1.0;
                    self.c_bg = [10, 16, 28, 255];
                    self.c_panel = [22, 32, 50, 240];
                    self.c_accent = [70, 190, 255, 255];
                    self.c_ok = [72, 212, 142, 255];
                    self.c_warn = [255, 140, 30, 255];
                    self.c_danger = [255, 62, 62, 255];
                }
            });

            card(&mut c[1], p, "ÖZELLEŞTİRME", 0.0, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Sıcaklık birimi");
                    ui.radio_value(&mut self.fahrenheit, false, "°C");
                    ui.radio_value(&mut self.fahrenheit, true, "°F");
                });
                ui.checkbox(&mut self.show_terminal, "Terminali göster");
                ui.checkbox(&mut self.blink, "Kritik uyarıları yanıp söndür");
                ui.horizontal(|ui| {
                    ui.label("Arayüz boyutu");
                    for s in [0.8_f32, 1.0, 1.2, 1.4] {
                        if ui.selectable_label((self.ui_scale - s).abs() < 0.01, format!("%{}", (s * 100.0) as i32)).clicked() {
                            self.ui_scale = s;
                        }
                    }
                });
            });
            card(&mut c[1], p, "KURULUM VE SENKRONİZASYON", 0.0, |ui| {
                ui.label(RichText::new(format!("Veri seti: {DATASET_PATH}")).monospace().size(13.0).color(p.dim));
                let b = egui::Button::new(RichText::new("VERİ SETİNİ YENİDEN OKU").strong().color(Color32::BLACK))
                    .fill(p.accent)
                    .min_size(Vec2::new(ui.available_width(), 56.0));
                if ui.add(b).clicked() {
                    self.sync_dataset();
                }
                ui.label(RichText::new(&self.sync_msg).color(p.ok));
                ui.label(
                    RichText::new("Dosya yoksa örnek veri seti otomatik oluşturulur. Dosyayı düzenleyip tekrar okutabilirsiniz (satır: anahtar=değer).")
                        .size(12.0)
                        .color(p.dim),
                );
            });
        });
    }
}
