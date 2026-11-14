use eframe::egui::{
    self, Align, Align2, Color32, CursorIcon, FontFamily, FontId, Frame, Key, Layout, Margin,
    Order, Pos2, Rect, RichText, Rounding, Sense, Shape, Stroke, Ui, Vec2,
};
use std::time::{Duration, Instant};

// ───────────────────────── Layer 1: Primitive Palette ─────────────────────────
mod palette {
    use eframe::egui::Color32;

    // Ink-Dark Slate Ramp (Standard Cockpit)
    pub const INK_950: Color32 = Color32::from_rgb(7, 9, 13);
    pub const INK_900: Color32 = Color32::from_rgb(11, 14, 20);
    pub const INK_850: Color32 = Color32::from_rgb(17, 22, 31);
    pub const INK_800: Color32 = Color32::from_rgb(23, 30, 42);
    pub const INK_750: Color32 = Color32::from_rgb(28, 36, 50);
    pub const INK_700: Color32 = Color32::from_rgb(38, 48, 65);
    pub const INK_600: Color32 = Color32::from_rgb(56, 70, 92);

    // High-Visor Solar Glare Ramp (AAA High Contrast)
    pub const VISOR_950: Color32 = Color32::from_rgb(0, 0, 0);
    pub const VISOR_900: Color32 = Color32::from_rgb(5, 7, 10);
    pub const VISOR_850: Color32 = Color32::from_rgb(12, 16, 22);
    pub const VISOR_800: Color32 = Color32::from_rgb(20, 26, 36);
    pub const VISOR_700: Color32 = Color32::from_rgb(92, 112, 142);

    // Warm Input Surface Tint
    pub const WARM_850: Color32 = Color32::from_rgb(22, 19, 15);
    pub const WARM_800: Color32 = Color32::from_rgb(31, 27, 20);
    pub const WARM_BORDER: Color32 = Color32::from_rgb(84, 62, 26);

    // Text Ramp
    pub const SLATE_50: Color32 = Color32::from_rgb(244, 246, 250);
    pub const SLATE_200: Color32 = Color32::from_rgb(203, 213, 225);
    pub const SLATE_400: Color32 = Color32::from_rgb(138, 149, 168);

    // Standard Aerospace Semantics
    pub const SKY_400: Color32 = Color32::from_rgb(56, 189, 248);
    pub const AMBER_400: Color32 = Color32::from_rgb(245, 158, 11);
    pub const MINT_400: Color32 = Color32::from_rgb(76, 195, 138);
    pub const CORAL_400: Color32 = Color32::from_rgb(239, 83, 80);

    // Colorblind-Safe (Okabe-Ito Inspired High-Distinction)
    pub const CB_TEAL: Color32 = Color32::from_rgb(0, 178, 148);
    pub const CB_BLUE: Color32 = Color32::from_rgb(86, 180, 233);
    pub const CB_ORANGE: Color32 = Color32::from_rgb(230, 159, 0);
    pub const CB_MAGENTA: Color32 = Color32::from_rgb(244, 114, 182);

    // Alert Surface
    pub const CRIMSON_900: Color32 = Color32::from_rgb(32, 14, 18);
    pub const CRIMSON_850: Color32 = Color32::from_rgb(44, 18, 23);
    pub const CB_ALERT_BG: Color32 = Color32::from_rgb(36, 15, 29);
    pub const CB_ALERT_HOVER: Color32 = Color32::from_rgb(48, 20, 38);
}

// ───────────────────────── Layer 2 & 3: Semantic & Component Tokens ─────────────────────────
#[derive(Clone, Copy, PartialEq, Eq)]
enum ContrastMode {
    Standard,
    HighVisor,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct AccessibilityConfig {
    contrast: ContrastMode,
    large_text: bool,
    colorblind_safe: bool,
    glove_targets: bool,
    reduced_motion: bool,
}

impl Default for AccessibilityConfig {
    fn default() -> Self {
        Self {
            contrast: ContrastMode::Standard,
            large_text: false,
            colorblind_safe: false,
            glove_targets: false,
            reduced_motion: false,
        }
    }
}

impl AccessibilityConfig {
    fn summary(self) -> String {
        let mut parts = Vec::new();
        if self.contrast == ContrastMode::HighVisor {
            parts.push("High-Visor");
        }
        if self.large_text {
            parts.push("Large Type");
        }
        if self.colorblind_safe {
            parts.push("CB-Safe");
        }
        if self.glove_targets {
            parts.push("Glove+");
        }
        if self.reduced_motion {
            parts.push("Static");
        }
        if parts.is_empty() {
            "Standard".to_string()
        } else {
            parts.join(" · ")
        }
    }
}

#[derive(Clone, Copy)]
struct Tokens {
    outer_bg: Color32,
    screen_bg: Color32,
    card_bg: Color32,
    card_hover: Color32,
    card_input_bg: Color32,
    card_input_hover: Color32,
    raised_bg: Color32,
    raised_hover: Color32,
    hairline: Color32,
    hairline_strong: Color32,
    hairline_input: Color32,
    accent_view: Color32,
    accent_input: Color32,
    nominal: Color32,
    caution: Color32,
    critical: Color32,
    alert_bg: Color32,
    alert_hover: Color32,
    text_main: Color32,
    text_body: Color32,
    text_muted: Color32,
    font_scale: f32,
    stroke_w: f32,
    button_h: f32,
    bezel_padding: f32,
}

impl Tokens {
    fn resolve(cfg: AccessibilityConfig) -> Self {
        let high = cfg.contrast == ContrastMode::HighVisor;
        let cb = cfg.colorblind_safe;

        Self {
            outer_bg: if high { palette::VISOR_950 } else { palette::INK_950 },
            screen_bg: if high { palette::VISOR_900 } else { palette::INK_900 },
            card_bg: if high { palette::VISOR_850 } else { palette::INK_850 },
            card_hover: if high { palette::VISOR_800 } else { palette::INK_800 },
            card_input_bg: palette::WARM_850,
            card_input_hover: palette::WARM_800,
            raised_bg: if high { palette::VISOR_800 } else { palette::INK_750 },
            raised_hover: if high { palette::INK_700 } else { palette::INK_700 },
            hairline: if high { palette::VISOR_700 } else { palette::INK_700 },
            hairline_strong: if high { palette::SLATE_200 } else { palette::INK_600 },
            hairline_input: if high { palette::AMBER_400 } else { palette::WARM_BORDER },
            accent_view: if cb { palette::CB_BLUE } else { palette::SKY_400 },
            accent_input: if cb { palette::CB_ORANGE } else { palette::AMBER_400 },
            nominal: if cb { palette::CB_TEAL } else { palette::MINT_400 },
            caution: if cb { palette::CB_ORANGE } else { palette::AMBER_400 },
            critical: if cb { palette::CB_MAGENTA } else { palette::CORAL_400 },
            alert_bg: if cb { palette::CB_ALERT_BG } else { palette::CRIMSON_900 },
            alert_hover: if cb { palette::CB_ALERT_HOVER } else { palette::CRIMSON_850 },
            text_main: if high { Color32::WHITE } else { palette::SLATE_50 },
            text_body: if high { palette::SLATE_50 } else { palette::SLATE_200 },
            text_muted: if high { palette::SLATE_200 } else { palette::SLATE_400 },
            font_scale: if cfg.large_text { 1.15 } else { 1.0 },
            stroke_w: if high { 1.5_f32 } else { 1.0_f32 },
            button_h: if cfg.glove_targets { 52.0 } else { 44.0 },
            bezel_padding: if cfg.glove_targets { 16.0 } else { 12.0 },
        }
    }

    fn sz(&self, base: f32) -> f32 {
        (base * self.font_scale).round()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StatusLevel {
    Nominal, // Circle
    Caution, // Triangle
    Critical, // Square
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([896.0, 504.0])
            .with_min_inner_size([640.0, 360.0])
            .with_title("AEGIS-BIO // Wrist Biomon"),
        ..Default::default()
    };

    eframe::run_native(
        "AEGIS-BIO Wrist Biomon",
        options,
        Box::new(|cc| Box::new(BiomonApp::new(cc))),
    )
}

// ───────────────────────── Domain State & Protocols ─────────────────────────
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Main,
    Input,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PanelId {
    Health,
    Hub,
    Tech,
    Hazard,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Launch,
    Orbit,
    Eva,
    ReEntry,
}

impl Phase {
    const ALL: [Phase; 4] = [Phase::Launch, Phase::Orbit, Phase::Eva, Phase::ReEntry];

    fn name(self) -> &'static str {
        match self {
            Self::Launch => "Launch",
            Self::Orbit => "Orbit",
            Self::Eva => "EVA",
            Self::ReEntry => "Re-entry",
        }
    }

    fn base_hr(self) -> i32 {
        match self {
            Self::Launch => 124,
            Self::Orbit => 88,
            Self::Eva => 138,
            Self::ReEntry => 132,
        }
    }

    fn hr_bounds(self) -> (i32, i32) {
        match self {
            Self::Launch => (100, 140),
            Self::Orbit => (55, 95),
            Self::Eva => (110, 155),
            Self::ReEntry => (90, 135),
        }
    }

    fn hr_range(self) -> &'static str {
        match self {
            Self::Launch => "100–140 bpm",
            Self::Orbit => "55–95 bpm",
            Self::Eva => "110–155 bpm",
            Self::ReEntry => "90–135 bpm",
        }
    }

    fn bp(self) -> &'static str {
        match self {
            Self::Launch => "132/84",
            Self::Orbit => "118/76",
            Self::Eva => "138/86",
            Self::ReEntry => "102/64",
        }
    }

    fn kcal_rate(self) -> f32 {
        match self {
            Self::Launch => 3.4,
            Self::Orbit => 1.8,
            Self::Eva => 6.8,
            Self::ReEntry => 3.1,
        }
    }

    fn rag_title(self) -> &'static str {
        match self {
            Self::Launch => "Ascent corridor (+3.8 Gx)",
            Self::Orbit => "Microgravity eye-pressure protection",
            Self::Eva => "EVA workload threshold active",
            Self::ReEntry => "Re-entry fluid load active (T-118 min)",
        }
    }

    fn rag_text(self) -> &'static str {
        match self {
            Self::Launch => "Heart rate (100–140 bpm) and breathing (20–30 brpm) are safe under launch G-load. Low-residue ration active.",
            Self::Orbit => "Keep daily sodium under 2,500 mg to reduce bone loss and headward fluid shift (SANS eye pressure).",
            Self::Eva => "138 bpm is safe in EVA corridor (110–155 bpm). Burn rate is 410 kcal/hr — sip from the In-Suit Drink Bag.",
            Self::ReEntry => "Sodium limit lifted for landing prep. Drink 1,150 mL water with 9x 1g salt tablets before +4 Gz descent.",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Hazard {
    Nominal,
    Radiation,
    HighCo2,
    Hypoxia,
}

impl Hazard {
    const ALL: [Hazard; 4] = [
        Hazard::Nominal,
        Hazard::Radiation,
        Hazard::HighCo2,
        Hazard::Hypoxia,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Nominal => "Nominal",
            Self::Radiation => "Solar radiation spike",
            Self::HighCo2 => "High suit CO2",
            Self::Hypoxia => "Low tissue oxygen",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Nominal => "All suit levels safe",
            Self::Radiation => "High solar radiation",
            Self::HighCo2 => "High CO2 (5.4 mmHg)",
            Self::Hypoxia => "Low oxygen (89%)",
        }
    }

    fn rad(self) -> &'static str {
        match self {
            Self::Radiation => "4.85 mSv/d",
            _ => "0.42 mSv/d",
        }
    }

    fn rad_val(self) -> &'static str {
        match self {
            Self::Radiation => "4.85",
            _ => "0.42",
        }
    }

    fn co2(self) -> &'static str {
        match self {
            Self::HighCo2 => "5.4 mmHg",
            Self::Hypoxia => "3.4 mmHg",
            _ => "2.8 mmHg",
        }
    }

    fn co2_val(self) -> &'static str {
        match self {
            Self::HighCo2 => "5.4",
            Self::Hypoxia => "3.4",
            _ => "2.8",
        }
    }

    fn o2(self) -> &'static str {
        match self {
            Self::HighCo2 => "96%",
            Self::Hypoxia => "89%",
            _ => "98%",
        }
    }

    fn sans(self) -> &'static str {
        match self {
            Self::HighCo2 => "Elevated",
            _ => "Low",
        }
    }

    fn rag_title(self) -> &'static str {
        match self {
            Self::Nominal => "Suit environment nominal",
            Self::Radiation => "Action: Return to radiation shelter",
            Self::HighCo2 => "Action: Inspect CO2 scrubber",
            Self::Hypoxia => "Action: Check oxygen valve",
        }
    }

    fn rag_text(self) -> &'static str {
        match self {
            Self::Nominal => "Radiation, scrubber CO2 (<4.0 mmHg), and tissue oxygen (≥92%) are within safe NASA-STD-3001 limits.",
            Self::Radiation => "Radiation exceeds 2.0 mSv/d safe limit. End surface EVA now and head 222 m NE to the storm shelter.",
            Self::HighCo2 => "High CO2 raises brain blood flow and eye pressure. Check LiOH scrubber cartridge and cap sodium at <1,800 mg.",
            Self::Hypoxia => "Tissue O2 dropped below 92%. Verify primary O2 regulator at 4.3 psi and switch to Backup O2 Pack if pressure falls.",
        }
    }
}

struct RationPlan {
    short: &'static str,
    name: &'static str,
    reason: String,
}

struct BiomonState {
    mode: Mode,
    phase: Phase,
    online: bool,
    buffered_packets: u32,
    meal_eaten_pct: u32,
    water_ml: u32,
    nacl_count: u32,
    hazard: Hazard,
    distance_m: u32,
    calorie_deficit: u32,
    sweat_deficit_ml: u32,
    live_hr: i32,
    active_panel: Option<PanelId>,
    a11y: AccessibilityConfig,
    pulse_phase: bool,
}

impl Default for BiomonState {
    fn default() -> Self {
        Self {
            mode: Mode::Main,
            phase: Phase::Orbit,
            online: true,
            buffered_packets: 0,
            meal_eaten_pct: 100,
            water_ml: 0,
            nacl_count: 0,
            hazard: Hazard::Nominal,
            distance_m: 222,
            calorie_deficit: 120,
            sweat_deficit_ml: 85,
            live_hr: 88,
            active_panel: None,
            a11y: AccessibilityConfig::default(),
            pulse_phase: false,
        }
    }
}

impl BiomonState {
    fn compute_ration(&self) -> RationPlan {
        if self.phase == Phase::ReEntry {
            return RationPlan {
                short: "1.15 L + 9x NaCl",
                name: "Re-entry hydration load (1,150 mL water + 9x 1g salt)",
                reason: "Restores blood plasma volume before gravity return to prevent fainting on landing.".into(),
            };
        }
        if self.hazard == Hazard::HighCo2 {
            return RationPlan {
                short: "Low-Na+ salmon",
                name: "Low-sodium salmon & quinoa pack (390 mg Na+)",
                reason: format!(
                    "High CO2 raises eye-pressure risk; holding back high-salt packs while covering {}% uneaten meal.",
                    100 - self.meal_eaten_pct
                ),
            };
        }
        if self.meal_eaten_pct < 100 || self.phase == Phase::Eva {
            return RationPlan {
                short: "High-K+ fajita",
                name: "High-potassium chicken fajita + electrolyte drink",
                reason: format!(
                    "Covers {}% uneaten meal ({} kcal and {} mL sweat loss) while staying under 2,500 mg daily sodium.",
                    100 - self.meal_eaten_pct,
                    self.calorie_deficit,
                    self.sweat_deficit_ml
                ),
            };
        }
        RationPlan {
            short: "Balanced ration #4",
            name: "ISS balanced ration #4 (turkey & vegetables)",
            reason: "Standard microgravity meal; maintains daily sodium below 2,500 mg.".into(),
        }
    }

    fn set_meal(&mut self, pct: u32) {
        self.meal_eaten_pct = pct;
        let eva_base = if self.phase == Phase::Eva { 410 } else { 120 };
        self.calorie_deficit = ((100 - pct) as f32 * 6.5).round() as u32 + eva_base;
    }

    fn set_phase(&mut self, phase: Phase) {
        self.phase = phase;
        self.live_hr = phase.base_hr();
        if phase == Phase::Eva {
            self.calorie_deficit = 410;
            self.sweat_deficit_ml = 850;
        }
    }
}

// ───────────────────────── Application Shell ─────────────────────────
struct BiomonApp {
    state: BiomonState,
    last_tick: Instant,
    prng: u64,
}

impl BiomonApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.style_mut(|s| {
            s.spacing.item_spacing = Vec2::new(8.0, 8.0);
            s.spacing.button_padding = Vec2::new(12.0, 8.0);
        });
        let app = Self {
            state: BiomonState::default(),
            last_tick: Instant::now(),
            prng: 0xAE61_5B10_2026_1114,
        };
        app.apply_visuals(&cc.egui_ctx, Tokens::resolve(app.state.a11y));
        app
    }

    fn apply_visuals(&self, ctx: &egui::Context, t: Tokens) {
        let mut v = egui::Visuals::dark();
        v.panel_fill = t.outer_bg;
        v.window_fill = t.screen_bg;
        v.override_text_color = Some(t.text_main);
        v.widgets.noninteractive.rounding = Rounding::same(8.0);
        v.widgets.inactive.rounding = Rounding::same(8.0);
        v.widgets.hovered.rounding = Rounding::same(8.0);
        v.widgets.active.rounding = Rounding::same(8.0);
        ctx.set_visuals(v);
    }

    fn next_rand(&mut self) -> u64 {
        self.prng = self
            .prng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.prng >> 32
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            if i.key_pressed(Key::C) {
                self.state.mode = match self.state.mode {
                    Mode::Main => Mode::Input,
                    Mode::Input => Mode::Main,
                };
            }
            if i.key_pressed(Key::Escape) {
                self.state.active_panel = None;
            }
            if i.key_pressed(Key::H) {
                self.state.a11y.contrast = match self.state.a11y.contrast {
                    ContrastMode::Standard => ContrastMode::HighVisor,
                    ContrastMode::HighVisor => ContrastMode::Standard,
                };
            }
            if self.state.active_panel.is_none() {
                if i.key_pressed(Key::Num1) {
                    self.state.active_panel = Some(PanelId::Health);
                } else if i.key_pressed(Key::Num2) {
                    self.state.active_panel = Some(PanelId::Hub);
                } else if i.key_pressed(Key::Num3) {
                    self.state.active_panel = Some(PanelId::Tech);
                } else if i.key_pressed(Key::Num4) {
                    self.state.active_panel = Some(PanelId::Hazard);
                }
            }
        });
    }

    fn tick_1hz(&mut self) {
        if self.last_tick.elapsed() < Duration::from_secs(1) {
            return;
        }
        self.last_tick = Instant::now();
        self.state.pulse_phase = !self.state.pulse_phase;

        let base = self.state.phase.base_hr()
            + if self.state.hazard == Hazard::Hypoxia { 14 } else { 0 };
        let jitter = (self.next_rand() % 3) as i32 - 1;
        self.state.live_hr = base + jitter;

        if self.state.phase == Phase::Eva && (self.next_rand() % 2 == 1) {
            self.state.distance_m += 1;
        }

        if !self.state.online {
            self.state.buffered_packets += 1;
        }
    }
}

impl eframe::App for BiomonApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ctx);
        self.tick_1hz();
        let t = Tokens::resolve(self.state.a11y);
        self.apply_visuals(ctx, t);
        ctx.request_repaint_after(Duration::from_millis(250));

        egui::CentralPanel::default()
            .frame(Frame::none().fill(t.outer_bg).inner_margin(Margin::same(0.0)))
            .show(ctx, |ui| {
                let avail = ui.available_rect_before_wrap();
                let margined = avail.shrink2(Vec2::new(avail.width() * 0.02, avail.height() * 0.02));
                let screen_rect = fit_16_by_9(margined);

                // Hardware display bezel
                let painter = ui.painter();
                painter.rect_filled(screen_rect, 16.0, t.screen_bg);
                painter.rect_stroke(screen_rect, 16.0, Stroke::new(t.stroke_w, t.hairline_strong));

                if let Some(panel) = self.state.active_panel {
                    self.render_detail_overlay(ui, screen_rect, panel, t);
                } else {
                    self.render_quadrant_grid(ui, ctx, screen_rect, t);
                }
            });
    }
}

impl BiomonApp {
    fn render_quadrant_grid(&mut self, ui: &mut Ui, ctx: &egui::Context, screen_rect: Rect, t: Tokens) {
        let center = screen_rect.center();
        let inner_r = (screen_rect.height() * 0.18).min(80.0);
        let ring_r = inner_r + 8.0;

        let pointer_in_cycle = ctx
            .input(|i| i.pointer.hover_pos())
            .is_some_and(|pos| pos.distance(center) <= ring_r);

        let gap = 12.0_f32;
        let grid_rect = screen_rect.shrink(gap);
        let qw = (grid_rect.width() - gap) / 2.0;
        let qh = (grid_rect.height() - gap) / 2.0;
        let clear_x = (qw * 0.16).max(52.0);
        let clear_y = (qh * 0.12).max(20.0);

        let rects = [
            (
                PanelId::Health,
                Rect::from_min_size(grid_rect.min, Vec2::new(qw, qh)),
                Margin {
                    left: t.bezel_padding + 2.0,
                    right: clear_x,
                    top: 12.0,
                    bottom: clear_y,
                },
            ),
            (
                PanelId::Hub,
                Rect::from_min_size(
                    Pos2::new(grid_rect.min.x + qw + gap, grid_rect.min.y),
                    Vec2::new(qw, qh),
                ),
                Margin {
                    left: clear_x,
                    right: t.bezel_padding + 2.0,
                    top: 12.0,
                    bottom: clear_y,
                },
            ),
            (
                PanelId::Tech,
                Rect::from_min_size(
                    Pos2::new(grid_rect.min.x, grid_rect.min.y + qh + gap),
                    Vec2::new(qw, qh),
                ),
                Margin {
                    left: t.bezel_padding + 2.0,
                    right: clear_x,
                    top: clear_y,
                    bottom: 12.0,
                },
            ),
            (
                PanelId::Hazard,
                Rect::from_min_size(
                    Pos2::new(grid_rect.min.x + qw + gap, grid_rect.min.y + qh + gap),
                    Vec2::new(qw, qh),
                ),
                Margin {
                    left: clear_x,
                    right: t.bezel_padding + 2.0,
                    top: clear_y,
                    bottom: 12.0,
                },
            ),
        ];

        for (panel_id, q_rect, pad) in rects {
            let is_alert = match panel_id {
                PanelId::Health => self.state.hazard == Hazard::Hypoxia,
                PanelId::Hazard => self.state.hazard != Hazard::Nominal,
                _ => false,
            };

            let resp = ui.interact(q_rect, ui.id().with(panel_id as u8), Sense::click());
            let hovered = resp.hovered() && !pointer_in_cycle;
            if hovered {
                ctx.set_cursor_icon(CursorIcon::PointingHand);
            }

            let is_input = self.state.mode == Mode::Input;
            let bg = if is_alert {
                if hovered { t.alert_hover } else { t.alert_bg }
            } else if is_input {
                if hovered { t.card_input_hover } else { t.card_input_bg }
            } else if hovered {
                t.card_hover
            } else {
                t.card_bg
            };

            let border_stroke = if is_alert {
                Stroke::new(t.stroke_w.max(1.5_f32), t.critical)
            } else if is_input {
                Stroke::new(
                    t.stroke_w,
                    if hovered { t.accent_input } else { t.hairline_input },
                )
            } else {
                Stroke::new(
                    t.stroke_w,
                    if hovered { t.hairline_strong } else { t.hairline },
                )
            };

            let painter = ui.painter();
            painter.rect_filled(q_rect, 12.0, bg);
            painter.rect_stroke(q_rect, 12.0, border_stroke);

            let content_rect = Rect::from_min_max(
                Pos2::new(q_rect.min.x + pad.left, q_rect.min.y + pad.top),
                Pos2::new(q_rect.max.x - pad.right, q_rect.max.y - pad.bottom),
            );
            ui.allocate_ui_at_rect(content_rect, |ui| {
                self.render_quadrant_content(ui, panel_id, t);
            });

            if resp.clicked() && !pointer_in_cycle {
                self.state.active_panel = Some(panel_id);
            }
        }

        // Dead-Center Circular Cycle Dial (fixed layout center)
        egui::Area::new(egui::Id::new("cycle_button"))
            .order(Order::Foreground)
            .fixed_pos(center - Vec2::splat(ring_r))
            .show(ctx, |ui| {
                let (ring_rect, _) = ui.allocate_exact_size(Vec2::splat(ring_r * 2.0), Sense::hover());
                let inner_rect = Rect::from_center_size(ring_rect.center(), Vec2::splat(inner_r * 2.0));
                let resp = ui.interact(inner_rect, ui.id().with("cycle_inner"), Sense::click());
                if resp.hovered() {
                    ctx.set_cursor_icon(CursorIcon::PointingHand);
                }

                let painter = ui.painter();
                let c = ring_rect.center();

                // Outer cutout bezel + subtle instrument cardinal ticks
                painter.circle(c, ring_r, t.screen_bg, Stroke::new(t.stroke_w, t.hairline));
                for k in 0..4 {
                    let a = k as f32 * std::f32::consts::FRAC_PI_2;
                    let d = Vec2::new(a.cos(), a.sin());
                    painter.line_segment(
                        [c + d * (ring_r - 4.0), c + d * (ring_r - 1.0)],
                        Stroke::new(1.0_f32, t.hairline_strong),
                    );
                }

                let (base_fill, hover_fill, ring_col, mode_lbl) = match self.state.mode {
                    Mode::Main => (t.card_bg, t.raised_bg, t.accent_view, "View mode"),
                    Mode::Input => (t.card_input_bg, t.card_input_hover, t.accent_input, "Edit mode"),
                };
                let fill_col = if resp.hovered() { hover_fill } else { base_fill };
                let stroke_w = if resp.hovered() {
                    t.stroke_w + 1.0_f32
                } else {
                    t.stroke_w + 0.5_f32
                };

                painter.circle(c, inner_r, fill_col, Stroke::new(stroke_w, ring_col));

                let icon_col = if self.state.mode == Mode::Input {
                    t.accent_input
                } else {
                    t.text_main
                };
                draw_refresh_icon(painter, Pos2::new(c.x, c.y - inner_r * 0.44), inner_r * 0.14, icon_col);

                painter.text(
                    Pos2::new(c.x, c.y - inner_r * 0.10),
                    Align2::CENTER_CENTER,
                    "Cycle",
                    FontId::proportional((inner_r * 0.19).clamp(12.0, 15.0) * t.font_scale),
                    t.text_main,
                );
                painter.text(
                    Pos2::new(c.x, c.y + inner_r * 0.15),
                    Align2::CENTER_CENTER,
                    mode_lbl,
                    FontId::proportional((inner_r * 0.13).clamp(10.0, 12.0) * t.font_scale),
                    ring_col,
                );

                // Phase + shortcut hint
                painter.text(
                    Pos2::new(c.x, c.y + inner_r * 0.46),
                    Align2::CENTER_CENTER,
                    self.state.phase.name(),
                    FontId::monospace((inner_r * 0.13).clamp(9.5, 11.5) * t.font_scale),
                    t.text_muted,
                );

                if resp.clicked() {
                    self.state.mode = match self.state.mode {
                        Mode::Main => Mode::Input,
                        Mode::Input => Mode::Main,
                    };
                }
            });
    }

    fn render_quadrant_content(&self, ui: &mut Ui, panel: PanelId, t: Tokens) {
        let is_input = self.state.mode == Mode::Input;
        let ration = self.state.compute_ration();

        let (num, title, footer_l, footer_r) = match panel {
            PanelId::Health => (
                "01",
                "Health & suit",
                format!("Deficit {} kcal · {} mL", self.state.calorie_deficit, self.state.sweat_deficit_ml),
                format!("BP {}", self.state.phase.bp()),
            ),
            PanelId::Hub => (
                "02",
                "Hub & navigation",
                "Bearing 042° NE".into(),
                "Lag 18m 42s".into(),
            ),
            PanelId::Tech => (
                "03",
                "Comms & display",
                format!("Queue {} pkts", self.state.buffered_packets),
                format!("A11y: {}", self.state.a11y.summary()),
            ),
            PanelId::Hazard => (
                "04",
                "Hazard & env",
                format!("SANS {}", self.state.hazard.sans()),
                "Press [H] Hi-Vis".into(),
            ),
        };

        // Zone 1: Header Strip
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(num)
                    .family(FontFamily::Monospace)
                    .size(t.sz(11.0))
                    .color(t.text_muted),
            );
            ui.label(RichText::new(title).size(t.sz(13.0)).color(t.text_body));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let (tag_txt, tag_col) = if is_input {
                    ("Edit", t.accent_input)
                } else {
                    ("Live", t.text_muted)
                };
                ui.label(RichText::new(tag_txt).size(t.sz(11.0)).color(tag_col));
            });
        });

        ui.add_space(4.0);
        ui.painter().line_segment(
            [
                Pos2::new(ui.min_rect().left(), ui.cursor().top()),
                Pos2::new(ui.max_rect().right(), ui.cursor().top()),
            ],
            Stroke::new(1.0_f32, t.hairline),
        );

        // Zone 2: Hero Telemetry / Control Body
        let body_h = (ui.available_height() - 24.0).max(40.0);
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), body_h),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.vertical(|ui| {
                    ui.add_space(4.0);
                    match panel {
                        PanelId::Health => {
                            if !is_input {
                                let (lvl, suit_col, suit_txt, suit_sub) = if self.state.hazard == Hazard::Hypoxia {
                                    (StatusLevel::Critical, t.critical, "Hypoxia alert", "Check O2 valve")
                                } else if self.state.phase == Phase::ReEntry {
                                    (StatusLevel::Caution, t.caution, "Fluid loading", "Drink + salt active")
                                } else {
                                    (StatusLevel::Nominal, t.nominal, "Suit safe", "Vitals in range")
                                };

                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            let beat_color = if !self.state.a11y.reduced_motion && self.state.pulse_phase {
                                                suit_col.gamma_multiply(0.8)
                                            } else {
                                                suit_col
                                            };
                                            draw_heart_icon(ui, 16.0, beat_color);
                                            ui.label(
                                                RichText::new(format!("{}", self.state.live_hr))
                                                    .family(FontFamily::Monospace)
                                                    .size(t.sz(25.0))
                                                    .color(t.text_main),
                                            );
                                            ui.label(
                                                RichText::new("bpm")
                                                    .family(FontFamily::Monospace)
                                                    .size(t.sz(11.0))
                                                    .color(t.text_muted),
                                            );
                                        });
                                        // Safe corridor bar — immediate visual glance of where BPM sits
                                        let (lo, hi) = self.state.phase.hr_bounds();
                                        draw_corridor_bar(
                                            ui,
                                            116.0,
                                            self.state.live_hr as f32,
                                            lo as f32,
                                            hi as f32,
                                            suit_col,
                                            t,
                                        );
                                        ui.label(
                                            RichText::new(format!("Safe {}", self.state.phase.hr_range()))
                                                .family(FontFamily::Monospace)
                                                .size(t.sz(10.5))
                                                .color(t.text_muted),
                                        );
                                    });

                                    ui.add_space(8.0);
                                    ui.vertical(|ui| {
                                        status_chip(ui, lvl, suit_txt, suit_col, t);
                                        ui.add_space(2.0);
                                        ui.label(RichText::new(suit_sub).size(t.sz(11.5)).color(t.text_body));
                                    });
                                });
                            } else {
                                ui.label(RichText::new("Log meal & landing fluids").size(t.sz(14.0)).color(t.accent_input));
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(format!("Last meal: {}% eaten", self.state.meal_eaten_pct))
                                        .size(t.sz(12.5))
                                        .color(t.text_main),
                                );
                                ui.label(
                                    RichText::new("Open to record food % or 1,150 mL + salt tablets.")
                                        .size(t.sz(11.5))
                                        .color(t.text_muted),
                                );
                            }
                        }
                        PanelId::Hub => {
                            if !is_input {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(format!("{}", self.state.distance_m))
                                                    .family(FontFamily::Monospace)
                                                    .size(t.sz(25.0))
                                                    .color(t.text_main),
                                            );
                                            ui.label(
                                                RichText::new("m to hub")
                                                    .family(FontFamily::Monospace)
                                                    .size(t.sz(11.0))
                                                    .color(t.text_muted),
                                            );
                                        });
                                        ui.label(
                                            RichText::new(format!("Phase: {}", self.state.phase.name()))
                                                .size(t.sz(12.5))
                                                .color(t.text_body),
                                        );
                                        ui.label(
                                            RichText::new(ration.short)
                                                .size(t.sz(11.5))
                                                .color(t.accent_view),
                                        );
                                    });
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        draw_compass_dial(ui, 44.0, 42.0_f32.to_radians(), t.accent_view, t);
                                    });
                                });
                            } else {
                                ui.label(RichText::new("Mission phase & hub sync").size(t.sz(14.0)).color(t.accent_input));
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(format!("Current phase: {}", self.state.phase.name()))
                                        .size(t.sz(12.5))
                                        .color(t.text_main),
                                );
                                ui.label(
                                    RichText::new("Open to switch phase, flush buffer, or set range.")
                                        .size(t.sz(11.5))
                                        .color(t.text_muted),
                                );
                            }
                        }
                        PanelId::Tech => {
                            if !is_input {
                                if self.state.hazard != Hazard::Nominal {
                                    status_chip(ui, StatusLevel::Critical, "1 action alert", t.critical, t);
                                } else {
                                    status_chip(ui, StatusLevel::Nominal, "No pending alerts", t.nominal, t);
                                }
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new("Local safety guide: NASA-STD-3001 ready")
                                        .size(t.sz(11.5))
                                        .color(t.text_body),
                                );
                                ui.add_space(2.0);
                                if self.state.online {
                                    status_chip(ui, StatusLevel::Nominal, "Hub link online", t.nominal, t);
                                } else {
                                    status_chip(ui, StatusLevel::Caution, "Offline · saving locally", t.caution, t);
                                }
                            } else {
                                ui.label(RichText::new("Comms, buffer & accessibility").size(t.sz(14.0)).color(t.accent_input));
                                ui.add_space(4.0);
                                let link_txt = if self.state.online {
                                    "Link: Online (surface LAN)".to_string()
                                } else {
                                    format!("Link: Offline ({} queued)", self.state.buffered_packets)
                                };
                                ui.label(RichText::new(link_txt).size(t.sz(12.0)).color(t.text_main));
                                ui.label(
                                    RichText::new("Open for LAN blackout test & astronaut display settings.")
                                        .size(t.sz(11.5))
                                        .color(t.text_muted),
                                );
                            }
                        }
                        PanelId::Hazard => {
                            if !is_input {
                                let (lvl, title_col) = match self.state.hazard {
                                    Hazard::Nominal => (StatusLevel::Nominal, t.nominal),
                                    Hazard::HighCo2 => (StatusLevel::Caution, t.caution),
                                    Hazard::Radiation | Hazard::Hypoxia => (StatusLevel::Critical, t.critical),
                                };
                                status_chip(ui, lvl, self.state.hazard.title(), title_col, t);
                                ui.add_space(6.0);
                                // 3-channel mini telemetry strip with clear limits
                                ui.horizontal(|ui| {
                                    mini_metric(
                                        ui,
                                        "Rad",
                                        self.state.hazard.rad_val(),
                                        "mSv/d",
                                        self.state.hazard == Hazard::Radiation,
                                        t,
                                    );
                                    ui.add_space(10.0);
                                    mini_metric(
                                        ui,
                                        "CO2",
                                        self.state.hazard.co2_val(),
                                        "mmHg",
                                        self.state.hazard == Hazard::HighCo2,
                                        t,
                                    );
                                    ui.add_space(10.0);
                                    mini_metric(
                                        ui,
                                        "O2",
                                        self.state.hazard.o2(),
                                        "sat",
                                        self.state.hazard == Hazard::Hypoxia,
                                        t,
                                    );
                                });
                            } else {
                                ui.label(RichText::new("Test hazard alerts").size(t.sz(14.0)).color(t.accent_input));
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(format!("Active: {}", self.state.hazard.label()))
                                        .size(t.sz(12.5))
                                        .color(t.text_main),
                                );
                                ui.label(
                                    RichText::new("Open to test radiation, high CO2, or low oxygen.")
                                        .size(t.sz(11.5))
                                        .color(t.text_muted),
                                );
                            }
                        }
                    }
                });
            },
        );

        // Zone 3: Footer Metadata
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(footer_l)
                    .family(FontFamily::Monospace)
                    .size(t.sz(10.5))
                    .color(t.text_muted),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(footer_r)
                        .family(FontFamily::Monospace)
                        .size(t.sz(10.5))
                        .color(t.text_muted),
                );
            });
        });
    }

    fn render_detail_overlay(&mut self, ui: &mut Ui, screen_rect: Rect, panel: PanelId, t: Tokens) {
        let is_input = self.state.mode == Mode::Input;
        let inner = screen_rect.shrink(16.0);
        let accent = if is_input { t.accent_input } else { t.accent_view };

        let (kicker, title) = match (panel, is_input) {
            (PanelId::Health, false) => ("01 · Health & suit", "Heart rate, hydration & ration plan"),
            (PanelId::Health, true) => ("01 · Edit mode", "Log meal & re-entry salt/water"),
            (PanelId::Hub, false) => ("02 · Hub & navigation", "Range to habitat & telemetry link"),
            (PanelId::Hub, true) => ("02 · Edit mode", "Mission phase & hub sync"),
            (PanelId::Tech, false) => ("03 · Comms & display", "Local safety engine, offline queue & display"),
            (PanelId::Tech, true) => ("03 · Edit mode", "Comms blackout & astronaut accessibility"),
            (PanelId::Hazard, false) => ("04 · Hazard & env", "Radiation, cabin gas & safety actions"),
            (PanelId::Hazard, true) => ("04 · Edit mode", "Test emergency hazard alerts"),
        };

        ui.allocate_ui_at_rect(inner, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(kicker)
                            .family(FontFamily::Monospace)
                            .size(t.sz(11.0))
                            .color(accent),
                    );
                    ui.label(RichText::new(title).size(t.sz(17.0)).color(t.text_main));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let btn_h = (t.button_h - 4.0).max(40.0);
                    let (btn_rect, resp) =
                        ui.allocate_exact_size(Vec2::new(148.0, btn_h), Sense::click());
                    if resp.hovered() {
                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                    }
                    let fill = if resp.hovered() { t.raised_hover } else { t.raised_bg };
                    let p = ui.painter();
                    p.rect(btn_rect, 8.0, fill, Stroke::new(t.stroke_w, t.hairline_strong));
                    p.text(
                        btn_rect.center(),
                        Align2::CENTER_CENTER,
                        "Return to grid [Esc]",
                        FontId::proportional(t.sz(12.5)),
                        t.text_main,
                    );
                    if resp.clicked() {
                        self.state.active_panel = None;
                    }
                });
            });

            ui.add_space(6.0);
            ui.painter().line_segment(
                [
                    Pos2::new(inner.left(), ui.cursor().top()),
                    Pos2::new(inner.right(), ui.cursor().top()),
                ],
                Stroke::new(1.0_f32, t.hairline),
            );
            ui.add_space(8.0);

            let body_h = (ui.available_height() - 28.0).max(100.0);
            egui::ScrollArea::vertical()
                .max_height(body_h)
                .auto_shrink([false, false])
                .show(ui, |ui| match panel {
                    PanelId::Health => self.overlay_health(ui, is_input, t),
                    PanelId::Hub => self.overlay_hub(ui, is_input, t),
                    PanelId::Tech => self.overlay_tech(ui, is_input, t),
                    PanelId::Hazard => self.overlay_hazard(ui, is_input, t),
                });

            ui.add_space(4.0);
            ui.painter().line_segment(
                [
                    Pos2::new(inner.left(), ui.cursor().top()),
                    Pos2::new(inner.right(), ui.cursor().top()),
                ],
                Stroke::new(1.0_f32, t.hairline),
            );
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("[C] Cycle mode · [1-4] Panels · [H] High-Visor contrast")
                        .family(FontFamily::Monospace)
                        .size(t.sz(10.0))
                        .color(t.text_muted),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("Profile: {}", self.state.a11y.summary()))
                            .family(FontFamily::Monospace)
                            .size(t.sz(10.0))
                            .color(t.text_muted),
                    );
                });
            });
        });
    }

    fn overlay_health(&mut self, ui: &mut Ui, is_input: bool, t: Tokens) {
        let p = self.state.phase;
        let ration = self.state.compute_ration();
        if !is_input {
            ui.columns(4, |c| {
                stat_card(
                    &mut c[0],
                    "Heart rate",
                    &format!("{} bpm", self.state.live_hr),
                    t.nominal,
                    &format!("Safe {}", p.hr_range()),
                    t,
                );
                stat_card(&mut c[1], "Blood pressure", p.bp(), t.text_main, "Cuffless optical", t);
                stat_card(
                    &mut c[2],
                    "Calorie deficit",
                    &format!("{} kcal", self.state.calorie_deficit),
                    t.caution,
                    &format!("{:.1} kcal/min burn", p.kcal_rate()),
                    t,
                );
                stat_card(
                    &mut c[3],
                    "Hydration loss",
                    &format!("{} mL", self.state.sweat_deficit_ml),
                    t.accent_view,
                    &format!("Logged +{} mL", self.state.water_ml),
                    t,
                );
            });
            ui.add_space(12.0);
            rag_box(ui, "Recommended ISS ration", Some(ration.name), &ration.reason, false, t);
            ui.add_space(8.0);
            rag_box(ui, p.rag_title(), None, p.rag_text(), p == Phase::ReEntry, t);
        } else {
            ui.label(RichText::new("Last meal eaten").size(t.sz(12.5)).color(t.text_body));
            ui.add_space(4.0);
            ui.columns(4, |c| {
                for (idx, pct) in [100_u32, 50, 25, 0].iter().enumerate() {
                    let sub = match *pct {
                        100 => "Full meal",
                        0 => "Skipped",
                        _ => "Partial",
                    };
                    if glove_button(
                        &mut c[idx],
                        &format!("{pct}% eaten"),
                        sub,
                        self.state.meal_eaten_pct == *pct,
                        t,
                    )
                    .clicked()
                    {
                        self.state.set_meal(*pct);
                    }
                }
            });
            ui.add_space(12.0);
            ui.label(
                RichText::new("Re-entry fluid & salt intake (goal: 1,150 mL + 9 tablets)")
                    .size(t.sz(12.5))
                    .color(t.text_body),
            );
            ui.add_space(4.0);
            ui.columns(2, |c| {
                if glove_button(
                    &mut c[0],
                    "+ 250 mL water",
                    &format!("{} / 1,150 mL", self.state.water_ml),
                    false,
                    t,
                )
                .clicked()
                {
                    self.state.water_ml += 250;
                    self.state.sweat_deficit_ml = self.state.sweat_deficit_ml.saturating_sub(250);
                }
                if glove_button(
                    &mut c[1],
                    "+ 1g salt tablet",
                    &format!("{} / 9 tablets", self.state.nacl_count),
                    false,
                    t,
                )
                .clicked()
                {
                    if self.state.nacl_count < 9 {
                        self.state.nacl_count += 1;
                    }
                }
            });
            ui.add_space(12.0);
            rag_box(ui, "Updated ration", Some(ration.name), &ration.reason, false, t);
        }
    }

    fn overlay_hub(&mut self, ui: &mut Ui, is_input: bool, t: Tokens) {
        let p = self.state.phase;
        if !is_input {
            ui.columns(4, |c| {
                stat_card(&mut c[0], "Hub range", &format!("{} m", self.state.distance_m), t.accent_view, "Bearing 042° NE", t);
                stat_card(&mut c[1], "Flight phase", p.name(), t.text_main, &format!("Safe {}", p.hr_range()), t);
                stat_card(&mut c[2], "Earth signal lag", "18m 42s", t.caution, "Suit runs locally", t);
                stat_card(
                    &mut c[3],
                    "Surface LAN",
                    if self.state.online { "Connected" } else { "Offline" },
                    if self.state.online { t.nominal } else { t.caution },
                    "192.168.1.10",
                    t,
                );
            });
            ui.add_space(12.0);
            rag_box(ui, p.rag_title(), None, p.rag_text(), false, t);
        } else {
            ui.label(RichText::new("Switch flight phase").size(t.sz(12.5)).color(t.text_body));
            ui.add_space(4.0);
            ui.columns(4, |c| {
                for (i, ph) in Phase::ALL.iter().enumerate() {
                    if glove_button(&mut c[i], ph.name(), ph.hr_range(), self.state.phase == *ph, t).clicked() {
                        self.state.set_phase(*ph);
                    }
                }
            });
            ui.add_space(12.0);
            ui.label(RichText::new("Hub link & waypoint").size(t.sz(12.5)).color(t.text_body));
            ui.add_space(4.0);
            ui.columns(2, |c| {
                if glove_button(
                    &mut c[0],
                    "Force sync to hub",
                    &format!("Send {} queued", self.state.buffered_packets),
                    false,
                    t,
                )
                .clicked()
                {
                    self.state.buffered_packets = 0;
                    self.state.online = true;
                }
                if glove_button(&mut c[1], "Set airlock waypoint", "145 m range", false, t).clicked() {
                    self.state.distance_m = 145;
                }
            });
        }
    }

    fn overlay_tech(&mut self, ui: &mut Ui, is_input: bool, t: Tokens) {
        if !is_input {
            ui.columns(4, |c| {
                stat_card(&mut c[0], "Safety engine", "1 Hz local", t.nominal, "No Earth lag", t);
                stat_card(
                    &mut c[1],
                    "Surface LAN",
                    if self.state.online { "Online" } else { "Offline" },
                    if self.state.online { t.nominal } else { t.caution },
                    "Auto-sync active",
                    t,
                );
                stat_card(
                    &mut c[2],
                    "Offline queue",
                    &format!("{} pkts", self.state.buffered_packets),
                    t.text_main,
                    "Saved on suit",
                    t,
                );
                stat_card(&mut c[3], "Medical guide DB", "64 MB local", t.accent_view, "NASA-STD-3001", t);
            });
            ui.add_space(12.0);
            rag_box(
                ui,
                "Offline crater safety & display profile",
                None,
                &format!(
                    "When crater walls block surface radio, your suit monitors NASA-STD-3001 thresholds locally every second and stores packets until reconnect. Active display accessibility: {}.",
                    self.state.a11y.summary()
                ),
                false,
                t,
            );
        } else {
            ui.label(RichText::new("Astronaut accessibility settings").size(t.sz(13.0)).color(t.text_main));
            ui.add_space(4.0);
            ui.columns(2, |c| {
                let hi_on = self.state.a11y.contrast == ContrastMode::HighVisor;
                if glove_button(
                    &mut c[0],
                    "High-visor sunlight contrast [H]",
                    if hi_on { "ON" } else { "OFF" },
                    hi_on,
                    t,
                )
                .clicked()
                {
                    self.state.a11y.contrast = if hi_on {
                        ContrastMode::Standard
                    } else {
                        ContrastMode::HighVisor
                    };
                }
                if glove_button(
                    &mut c[1],
                    "Large text size",
                    if self.state.a11y.large_text { "115%" } else { "100%" },
                    self.state.a11y.large_text,
                    t,
                )
                .clicked()
                {
                    self.state.a11y.large_text = !self.state.a11y.large_text;
                }
            });
            ui.add_space(8.0);
            ui.columns(2, |c| {
                if glove_button(
                    &mut c[0],
                    "Colorblind-safe + shape cues",
                    if self.state.a11y.colorblind_safe { "ON" } else { "OFF" },
                    self.state.a11y.colorblind_safe,
                    t,
                )
                .clicked()
                {
                    self.state.a11y.colorblind_safe = !self.state.a11y.colorblind_safe;
                }
                if glove_button(
                    &mut c[1],
                    "Extra glove button size",
                    if self.state.a11y.glove_targets { "52 px" } else { "44 px" },
                    self.state.a11y.glove_targets,
                    t,
                )
                .clicked()
                {
                    self.state.a11y.glove_targets = !self.state.a11y.glove_targets;
                }
            });
            ui.add_space(8.0);
            if glove_button(
                ui,
                "Reduced motion (static pulse indicator)",
                if self.state.a11y.reduced_motion { "ON" } else { "OFF" },
                self.state.a11y.reduced_motion,
                t,
            )
            .clicked()
            {
                self.state.a11y.reduced_motion = !self.state.a11y.reduced_motion;
            }

            ui.add_space(12.0);
            ui.label(RichText::new("LAN blackout test & local queue").size(t.sz(12.5)).color(t.text_body));
            ui.add_space(4.0);
            let label = if self.state.online {
                "Simulate crater radio blackout"
            } else {
                "Reconnect radio & flush packets"
            };
            let right = if self.state.online {
                "Online"
            } else {
                "Offline · buffering"
            };
            if glove_button(ui, label, right, !self.state.online, t).clicked() {
                self.state.online = !self.state.online;
                if self.state.online {
                    self.state.buffered_packets = 0;
                }
            }
            ui.add_space(8.0);
            stat_card(
                ui,
                "Queued telemetry frames",
                &format!("{} frames", self.state.buffered_packets),
                t.accent_view,
                "Saves 1 frame/s offline; sends automatically on reconnect.",
                t,
            );
        }
    }

    fn overlay_hazard(&mut self, ui: &mut Ui, is_input: bool, t: Tokens) {
        let h = self.state.hazard;
        if !is_input {
            ui.columns(4, |c| {
                stat_card(
                    &mut c[0],
                    "Radiation",
                    h.rad(),
                    if h == Hazard::Radiation { t.critical } else { t.nominal },
                    "Limit < 2.0 mSv/d",
                    t,
                );
                stat_card(
                    &mut c[1],
                    "Suit CO2",
                    h.co2(),
                    if h == Hazard::HighCo2 { t.caution } else { t.nominal },
                    "Limit < 4.0 mmHg",
                    t,
                );
                stat_card(
                    &mut c[2],
                    "Tissue oxygen",
                    h.o2(),
                    if h == Hazard::Hypoxia { t.critical } else { t.nominal },
                    "Safe >= 92%",
                    t,
                );
                stat_card(&mut c[3], "Surface wind", "3 m/s NE", t.text_main, "Visibility clear", t);
            });
            ui.add_space(12.0);
            rag_box(ui, h.rag_title(), None, h.rag_text(), h != Hazard::Nominal, t);
        } else {
            ui.label(RichText::new("Select hazard scenario to test").size(t.sz(12.5)).color(t.text_body));
            ui.add_space(4.0);
            for pair in Hazard::ALL.chunks(2) {
                ui.columns(2, |c| {
                    for (i, hk) in pair.iter().enumerate() {
                        let sub = if *hk == Hazard::Nominal { "Clear alert" } else { "Trigger alert" };
                        if glove_button(&mut c[i], hk.label(), sub, self.state.hazard == *hk, t).clicked() {
                            self.state.hazard = *hk;
                        }
                    }
                });
                ui.add_space(8.0);
            }
            ui.add_space(4.0);
            rag_box(ui, h.rag_title(), None, h.rag_text(), h != Hazard::Nominal, t);
        }
    }
}

// ───────────────────────── Accessible Instrument Primitives ─────────────────────────
fn fit_16_by_9(bounds: Rect) -> Rect {
    let target = 16.0_f32 / 9.0_f32;
    let (w, h) = if bounds.width() / bounds.height() > target {
        (bounds.height() * target, bounds.height())
    } else {
        (bounds.width(), bounds.width() / target)
    };
    Rect::from_center_size(bounds.center(), Vec2::new(w, h))
}

fn draw_status_shape(painter: &egui::Painter, center: Pos2, level: StatusLevel, color: Color32) {
    match level {
        StatusLevel::Nominal => {
            painter.circle_filled(center, 4.0, color);
        }
        StatusLevel::Caution => {
            let pts = vec![
                Pos2::new(center.x, center.y - 4.5),
                Pos2::new(center.x + 4.5, center.y + 3.5),
                Pos2::new(center.x - 4.5, center.y + 3.5),
            ];
            painter.add(Shape::convex_polygon(pts, color, Stroke::NONE));
        }
        StatusLevel::Critical => {
            let rect = Rect::from_center_size(center, Vec2::splat(8.0));
            painter.rect_filled(rect, 1.5, color);
        }
    }
}

fn status_chip(ui: &mut Ui, level: StatusLevel, label: &str, color: Color32, t: Tokens) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
        draw_status_shape(ui.painter(), r.center(), level, color);
        ui.label(RichText::new(label).size(t.sz(13.5)).color(color));
    });
}

fn draw_corridor_bar(
    ui: &mut Ui,
    width: f32,
    val: f32,
    lo: f32,
    hi: f32,
    marker_col: Color32,
    t: Tokens,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 8.0), Sense::hover());
    let p = ui.painter();
    let track = Rect::from_min_size(Pos2::new(rect.left(), rect.center().y - 2.0), Vec2::new(width, 4.0));
    p.rect_filled(track, 2.0, t.hairline);

    // Safe middle corridor zone
    let safe_left = track.left() + width * 0.20;
    let safe_w = width * 0.60;
    let safe_rect = Rect::from_min_size(Pos2::new(safe_left, track.top()), Vec2::new(safe_w, 4.0));
    p.rect_filled(safe_rect, 2.0, t.hairline_strong);

    let span = (hi - lo).max(1.0);
    let norm = ((val - lo) / span).clamp(0.0, 1.0);
    let x = safe_left + safe_w * norm;
    p.circle_filled(Pos2::new(x, track.center().y), 3.5, marker_col);
}

fn mini_metric(ui: &mut Ui, name: &str, val: &str, unit: &str, alerted: bool, t: Tokens) {
    let col = if alerted { t.critical } else { t.text_main };
    let bar_col = if alerted { t.critical } else { t.nominal };
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::new(3.0, 14.0), Sense::hover());
            ui.painter().rect_filled(r, 1.0, bar_col);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(format!("{name} {val}"))
                        .family(FontFamily::Monospace)
                        .size(t.sz(11.5))
                        .color(col),
                );
                ui.label(
                    RichText::new(unit)
                        .family(FontFamily::Monospace)
                        .size(t.sz(9.5))
                        .color(t.text_muted),
                );
            });
        });
    });
}

fn stat_card(ui: &mut Ui, label: &str, value: &str, val_color: Color32, note: &str, t: Tokens) {
    Frame::none()
        .fill(t.card_bg)
        .stroke(Stroke::new(t.stroke_w, t.hairline))
        .rounding(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(label).size(t.sz(11.5)).color(t.text_muted));
            ui.add_space(2.0);
            ui.label(
                RichText::new(value)
                    .family(FontFamily::Monospace)
                    .size(t.sz(18.0))
                    .color(val_color),
            );
            ui.add_space(2.0);
            ui.label(
                RichText::new(note)
                    .family(FontFamily::Monospace)
                    .size(t.sz(10.5))
                    .color(t.text_muted),
            );
        });
}

fn rag_box(ui: &mut Ui, header: &str, headline: Option<&str>, body: &str, danger: bool, t: Tokens) {
    let (bg, stroke, bar_col, hdr_col) = if danger {
        (t.alert_bg, Stroke::new(t.stroke_w, t.critical), t.critical, t.critical)
    } else {
        (t.card_bg, Stroke::new(t.stroke_w, t.hairline), t.accent_view, t.accent_view)
    };

    let resp = Frame::none()
        .fill(bg)
        .stroke(stroke)
        .rounding(8.0)
        .inner_margin(Margin {
            left: 16.0,
            right: 12.0,
            top: 10.0,
            bottom: 10.0,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(header).size(t.sz(12.0)).color(hdr_col));
            if let Some(hl) = headline {
                ui.add_space(2.0);
                ui.label(RichText::new(hl).size(t.sz(14.0)).color(t.text_main));
            }
            ui.add_space(4.0);
            ui.label(RichText::new(body).size(t.sz(12.5)).color(t.text_body));
        });

    let r = resp.response.rect;
    ui.painter().line_segment(
        [Pos2::new(r.left() + 1.5, r.top() + 8.0), Pos2::new(r.left() + 1.5, r.bottom() - 8.0)],
        Stroke::new(2.5_f32, bar_col),
    );
}

fn glove_button(ui: &mut Ui, left: &str, right: &str, selected: bool, t: Tokens) -> egui::Response {
    let (bg, border, text_col) = if selected {
        (t.card_input_hover, t.accent_input, t.accent_input)
    } else {
        (t.raised_bg, t.hairline_strong, t.text_main)
    };
    let desired = Vec2::new(ui.available_width(), t.button_h);
    let (rect, resp) = ui.allocate_exact_size(desired, Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    let fill = if resp.hovered() && !selected {
        t.raised_hover
    } else {
        bg
    };
    let p = ui.painter();
    p.rect(rect, 8.0, fill, Stroke::new(t.stroke_w, border));
    p.text(
        Pos2::new(rect.left() + 12.0, rect.center().y),
        Align2::LEFT_CENTER,
        left,
        FontId::proportional(t.sz(13.0)),
        text_col,
    );
    p.text(
        Pos2::new(rect.right() - 12.0, rect.center().y),
        Align2::RIGHT_CENTER,
        right,
        FontId::monospace(t.sz(11.0)),
        t.text_muted,
    );
    resp
}

fn draw_heart_icon(ui: &mut Ui, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let c = rect.center();
    let mut pts = Vec::with_capacity(48);
    for i in 0..48 {
        let t = i as f32 / 48.0 * std::f32::consts::TAU;
        let x = 16.0 * t.sin().powi(3);
        let y = 13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
        pts.push(Pos2::new(c.x + x * size / 36.0, c.y - (y + 2.0) * size / 36.0));
    }
    ui.painter().add(Shape::closed_line(pts, Stroke::new(1.5_f32, color)));
}

fn draw_compass_dial(ui: &mut Ui, size: f32, bearing_rad: f32, color: Color32, t: Tokens) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let c = rect.center();
    let r = size * 0.42;
    let p = ui.painter();
    p.circle_stroke(c, r, Stroke::new(t.stroke_w, t.hairline_strong));

    for k in 0..4 {
        let a = k as f32 * std::f32::consts::FRAC_PI_2;
        let d = Vec2::new(a.sin(), -a.cos());
        p.line_segment([c + d * (r - 3.0), c + d * r], Stroke::new(1.0_f32, t.text_muted));
    }

    let dir = Vec2::new(bearing_rad.sin(), -bearing_rad.cos());
    let tip = c + dir * (r * 0.78);
    p.line_segment([c, tip], Stroke::new(1.8_f32, color));
    let perp = Vec2::new(-dir.y, dir.x);
    let head1 = tip - dir * 5.0 + perp * 3.5;
    let head2 = tip - dir * 5.0 - perp * 3.5;
    p.line_segment([tip, head1], Stroke::new(1.8_f32, color));
    p.line_segment([tip, head2], Stroke::new(1.8_f32, color));
    p.circle_filled(c, 2.0, color);
}

fn draw_refresh_icon(painter: &egui::Painter, center: Pos2, r: f32, color: Color32) {
    let steps = 20;
    let mut pts = Vec::with_capacity(steps);
    let start = -0.3_f32;
    let span = 4.6_f32;
    for i in 0..steps {
        let a = start + span * (i as f32 / (steps - 1) as f32);
        pts.push(Pos2::new(center.x + a.cos() * r, center.y + a.sin() * r));
    }
    painter.add(Shape::line(pts.clone(), Stroke::new(1.5_f32, color)));
    if let Some(&end) = pts.last() {
        painter.line_segment([end, Pos2::new(end.x - 4.0, end.y - 1.5)], Stroke::new(1.5_f32, color));
        painter.line_segment([end, Pos2::new(end.x - 1.0, end.y + 4.0)], Stroke::new(1.5_f32, color));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_visor_and_colorblind_tokens_resolve_distinctly() {
        let std_t = Tokens::resolve(AccessibilityConfig::default());
        let a11y_t = Tokens::resolve(AccessibilityConfig {
            contrast: ContrastMode::HighVisor,
            large_text: true,
            colorblind_safe: true,
            glove_targets: true,
            reduced_motion: true,
        });

        assert_ne!(std_t.screen_bg, a11y_t.screen_bg);
        assert_ne!(std_t.nominal, a11y_t.nominal);
        assert_eq!(a11y_t.button_h, 52.0);
        assert!(a11y_t.sz(12.0) > std_t.sz(12.0));
    }

    #[test]
    fn reentry_phase_triggers_isotonic_ration() {
        let mut s = BiomonState::default();
        s.set_phase(Phase::ReEntry);
        let plan = s.compute_ration();
        assert_eq!(plan.short, "1.15 L + 9x NaCl");
        assert_eq!(s.live_hr, Phase::ReEntry.base_hr());
    }
}
