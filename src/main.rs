use eframe::egui::{
    self, Align, Align2, Color32, CursorIcon, FontFamily, FontId, Frame, Id, Key, Layout, Margin,
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
    pub const INK_800: Color32 = Color32::from_rgb(24, 31, 44);
    pub const INK_750: Color32 = Color32::from_rgb(29, 38, 53);
    pub const INK_700: Color32 = Color32::from_rgb(38, 48, 65);
    pub const INK_600: Color32 = Color32::from_rgb(58, 73, 96);

    // High-Visor Solar Glare Ramp (AAA High Contrast)
    pub const VISOR_950: Color32 = Color32::from_rgb(0, 0, 0);
    pub const VISOR_900: Color32 = Color32::from_rgb(5, 7, 10);
    pub const VISOR_850: Color32 = Color32::from_rgb(12, 16, 22);
    pub const VISOR_800: Color32 = Color32::from_rgb(22, 28, 39);
    pub const VISOR_700: Color32 = Color32::from_rgb(96, 118, 150);

    // Warm Input Surface Tint
    pub const WARM_850: Color32 = Color32::from_rgb(22, 19, 15);
    pub const WARM_800: Color32 = Color32::from_rgb(32, 27, 20);
    pub const WARM_BORDER: Color32 = Color32::from_rgb(94, 68, 28);

    // Text Ramp
    pub const SLATE_50: Color32 = Color32::from_rgb(244, 246, 250);
    pub const SLATE_200: Color32 = Color32::from_rgb(203, 213, 225);
    pub const SLATE_400: Color32 = Color32::from_rgb(138, 149, 168);

    // Distinct Per-Quadrant Subsystem Signatures
    pub const HEALTH_EMERALD: Color32 = Color32::from_rgb(52, 211, 153); // 01 Health
    pub const HUB_CYAN: Color32 = Color32::from_rgb(56, 189, 248); // 02 Navigation
    pub const COMMS_COBALT: Color32 = Color32::from_rgb(129, 140, 248); // 03 Comms
    pub const HAZARD_AMBER: Color32 = Color32::from_rgb(245, 158, 11); // 04 Env / Caution

    // Semantic Status
    pub const MINT_400: Color32 = Color32::from_rgb(76, 195, 138);
    pub const AMBER_400: Color32 = Color32::from_rgb(245, 158, 11);
    pub const CORAL_400: Color32 = Color32::from_rgb(239, 83, 80);

    // Colorblind-Safe (Okabe-Ito Distinct)
    pub const CB_TEAL: Color32 = Color32::from_rgb(0, 178, 148);
    pub const CB_BLUE: Color32 = Color32::from_rgb(86, 180, 233);
    pub const CB_INDIGO: Color32 = Color32::from_rgb(148, 163, 255);
    pub const CB_ORANGE: Color32 = Color32::from_rgb(230, 159, 0);
    pub const CB_MAGENTA: Color32 = Color32::from_rgb(244, 114, 182);

    // Alert Surface
    pub const CRIMSON_900: Color32 = Color32::from_rgb(32, 14, 18);
    pub const CRIMSON_850: Color32 = Color32::from_rgb(46, 19, 24);
    pub const CB_ALERT_BG: Color32 = Color32::from_rgb(36, 15, 29);
    pub const CB_ALERT_HOVER: Color32 = Color32::from_rgb(50, 20, 40);
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
            parts.push("Large");
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
    quad_health: Color32,
    quad_hub: Color32,
    quad_comms: Color32,
    quad_hazard: Color32,
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
    anim_secs: f32,
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
            raised_hover: palette::INK_700,
            hairline: if high { palette::VISOR_700 } else { palette::INK_700 },
            hairline_strong: if high { palette::SLATE_200 } else { palette::INK_600 },
            hairline_input: if high { palette::AMBER_400 } else { palette::WARM_BORDER },
            accent_view: if cb { palette::CB_BLUE } else { palette::HUB_CYAN },
            accent_input: if cb { palette::CB_ORANGE } else { palette::AMBER_400 },
            quad_health: if cb { palette::CB_TEAL } else { palette::HEALTH_EMERALD },
            quad_hub: if cb { palette::CB_BLUE } else { palette::HUB_CYAN },
            quad_comms: if cb { palette::CB_INDIGO } else { palette::COMMS_COBALT },
            quad_hazard: if cb { palette::CB_ORANGE } else { palette::HAZARD_AMBER },
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
            anim_secs: if cfg.reduced_motion { 0.0 } else { 0.20 },
        }
    }

    fn sz(&self, base: f32) -> f32 {
        (base * self.font_scale).round()
    }

    fn quad_accent(&self, panel: PanelId) -> Color32 {
        match panel {
            PanelId::Health => self.quad_health,
            PanelId::Hub => self.quad_hub,
            PanelId::Tech => self.quad_comms,
            PanelId::Hazard => self.quad_hazard,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StatusLevel {
    Nominal,  // Circle
    Caution,  // Triangle
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
            Self::Nominal => "All levels safe",
            Self::Radiation => "Radiation alert",
            Self::HighCo2 => "High CO2",
            Self::Hypoxia => "Low O2",
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

    fn rad_ratio(self) -> f32 {
        match self {
            Self::Radiation => 0.96,
            _ => 0.21,
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

    fn co2_ratio(self) -> f32 {
        match self {
            Self::HighCo2 => 0.90,
            Self::Hypoxia => 0.56,
            _ => 0.46,
        }
    }

    fn o2(self) -> &'static str {
        match self {
            Self::HighCo2 => "96%",
            Self::Hypoxia => "89%",
            _ => "98%",
        }
    }

    fn o2_ratio(self) -> f32 {
        match self {
            Self::HighCo2 => 0.90,
            Self::Hypoxia => 0.40,
            _ => 0.96,
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
    start_instant: Instant,
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
            start_instant: Instant::now(),
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

        // Smooth 30 Hz UI update for fluid card hover & suit heartbeat unless Reduced Motion is on
        let refresh_ms = if self.state.a11y.reduced_motion { 250 } else { 33 };
        ctx.request_repaint_after(Duration::from_millis(refresh_ms));

        egui::CentralPanel::default()
            .frame(Frame::none().fill(t.outer_bg).inner_margin(Margin::same(0.0)))
            .show(ctx, |ui| {
                let avail = ui.available_rect_before_wrap();
                let margined = avail.shrink2(Vec2::new(avail.width() * 0.02, avail.height() * 0.02));
                let screen_rect = fit_16_by_9(margined);

                let painter = ui.painter();
                painter.rect_filled(screen_rect, 16.0, t.screen_bg);
                painter.rect_stroke(screen_rect, 16.0, Stroke::new(t.stroke_w, t.hairline_strong));

                if let Some(panel) = self.state.active_panel {
                    self.render_detail_overlay(ui, ctx, screen_rect, panel, t);
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

        // Smooth Main <-> Input mode blend factor
        let mode_t = ctx.animate_bool_with_time(
            Id::new("mode_blend"),
            self.state.mode == Mode::Input,
            t.anim_secs,
        );

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
                    left: t.bezel_padding + 6.0,
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
                    right: t.bezel_padding + 6.0,
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
                    left: t.bezel_padding + 6.0,
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
                    right: t.bezel_padding + 6.0,
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

            let hover_t = ctx.animate_bool_with_time(
                Id::new(("quad_hover", panel_id as u8)),
                hovered,
                t.anim_secs,
            );

            let quad_col = t.quad_accent(panel_id);
            let main_bg = lerp_color(t.card_bg, t.card_hover, hover_t);
            let input_bg = lerp_color(t.card_input_bg, t.card_input_hover, hover_t);
            let normal_bg = lerp_color(main_bg, input_bg, mode_t);
            let bg = if is_alert {
                lerp_color(t.alert_bg, t.alert_hover, hover_t)
            } else {
                normal_bg
            };

            let normal_border = lerp_color(
                lerp_color(t.hairline, quad_col.gamma_multiply(0.55), hover_t),
                lerp_color(t.hairline_input, t.accent_input, hover_t),
                mode_t,
            );
            let border_col = if is_alert { t.critical } else { normal_border };
            let border_w = t.stroke_w + hover_t * 0.5_f32;

            let painter = ui.painter();
            painter.rect_filled(q_rect, 12.0, bg);
            painter.rect_stroke(q_rect, 12.0, Stroke::new(border_w, border_col));

            // Subsystem color-identity signature bar (lets astronauts recognize quadrant by color without reading)
            let bar_col = if is_alert {
                t.critical
            } else {
                lerp_color(quad_col, t.accent_input, mode_t * 0.45)
            };
            let is_left_side = matches!(panel_id, PanelId::Health | PanelId::Tech);
            let bar_x = if is_left_side {
                q_rect.left() + 4.0
            } else {
                q_rect.right() - 7.0
            };
            let bar_rect = Rect::from_min_max(
                Pos2::new(bar_x, q_rect.top() + 12.0),
                Pos2::new(bar_x + 3.0, q_rect.bottom() - 12.0),
            );
            painter.rect_filled(bar_rect, 2.0, bar_col);

            let content_rect = Rect::from_min_max(
                Pos2::new(q_rect.min.x + pad.left, q_rect.min.y + pad.top),
                Pos2::new(q_rect.max.x - pad.right, q_rect.max.y - pad.bottom),
            );
            ui.allocate_ui_at_rect(content_rect, |ui| {
                self.render_quadrant_content(ui, ctx, panel_id, t);
            });

            if resp.clicked() && !pointer_in_cycle {
                self.state.active_panel = Some(panel_id);
            }
        }

        // Dead-Center Circular Cycle Dial (layout anchor)
        egui::Area::new(Id::new("cycle_button"))
            .order(Order::Foreground)
            .fixed_pos(center - Vec2::splat(ring_r + 4.0))
            .show(ctx, |ui| {
                let alloc_r = ring_r + 4.0;
                let (ring_rect, _) = ui.allocate_exact_size(Vec2::splat(alloc_r * 2.0), Sense::hover());
                let inner_rect = Rect::from_center_size(ring_rect.center(), Vec2::splat(inner_r * 2.0));
                let resp = ui.interact(inner_rect, ui.id().with("cycle_inner"), Sense::click());
                if resp.hovered() {
                    ctx.set_cursor_icon(CursorIcon::PointingHand);
                }

                let c_hover = ctx.animate_bool_with_time(Id::new("cycle_hover"), resp.hovered(), t.anim_secs);
                let painter = ui.painter();
                let c = ring_rect.center();

                painter.circle(c, ring_r, t.screen_bg, Stroke::new(t.stroke_w, t.hairline));
                for k in 0..4 {
                    let a = k as f32 * std::f32::consts::FRAC_PI_2;
                    let d = Vec2::new(a.cos(), a.sin());
                    painter.line_segment(
                        [c + d * (ring_r - 4.0), c + d * (ring_r - 1.0)],
                        Stroke::new(1.0_f32, t.hairline_strong),
                    );
                }

                let main_fill = lerp_color(t.card_bg, t.raised_bg, c_hover);
                let edit_fill = lerp_color(t.card_input_bg, t.card_input_hover, c_hover);
                let fill_col = lerp_color(main_fill, edit_fill, mode_t);
                let ring_col = lerp_color(t.accent_view, t.accent_input, mode_t);
                let anim_r = inner_r + c_hover * 2.0;
                let stroke_w = t.stroke_w + 0.5_f32 + c_hover * 1.0_f32;

                painter.circle(c, anim_r, fill_col, Stroke::new(stroke_w, ring_col));

                let mode_lbl = if self.state.mode == Mode::Input {
                    "Edit mode"
                } else {
                    "View mode"
                };
                let icon_col = lerp_color(t.text_main, t.accent_input, mode_t);
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

    fn render_quadrant_content(&self, ui: &mut Ui, ctx: &egui::Context, panel: PanelId, t: Tokens) {
        let is_input = self.state.mode == Mode::Input;
        let ration = self.state.compute_ration();
        let accent = t.quad_accent(panel);

        let (num, title, footer_l, footer_r) = match panel {
            PanelId::Health => (
                "01",
                "Health & suit",
                format!("BP {}", self.state.phase.bp()),
                format!("-{} kcal · {} mL", self.state.calorie_deficit, self.state.sweat_deficit_ml),
            ),
            PanelId::Hub => (
                "02",
                "Hub & nav",
                "042° NE".into(),
                "Lag 18m 42s".into(),
            ),
            PanelId::Tech => (
                "03",
                "Comms & display",
                format!("{} queued", self.state.buffered_packets),
                self.state.a11y.summary(),
            ),
            PanelId::Hazard => (
                "04",
                "Hazard & env",
                format!("SANS {}", self.state.hazard.sans()),
                "[H] Hi-Vis".into(),
            ),
        };

        // Zone 1: Color-coded subsystem header (minimal reading needed)
        ui.horizontal(|ui| {
            let tag_fill = accent.gamma_multiply(0.16);
            Frame::none()
                .fill(tag_fill)
                .rounding(4.0)
                .inner_margin(Margin::symmetric(5.0, 1.0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(num)
                            .family(FontFamily::Monospace)
                            .size(t.sz(10.5))
                            .color(accent),
                    );
                });
            ui.label(RichText::new(title).size(t.sz(13.0)).color(t.text_main));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let (tag_txt, tag_col) = if is_input {
                    ("Edit", t.accent_input)
                } else {
                    ("Live", accent)
                };
                ui.label(RichText::new(tag_txt).size(t.sz(10.5)).color(tag_col));
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

        // Smooth continuous pulse factor (0..1) for live cardiac rhythm
        let pulse = if self.state.a11y.reduced_motion {
            0.0
        } else {
            let phase = self.state.start_instant.elapsed().as_secs_f32()
                * (self.state.live_hr.max(40) as f32 / 60.0)
                * std::f32::consts::TAU;
            (phase.sin() * 0.5 + 0.5).powi(2)
        };

        let body_h = (ui.available_height() - 22.0).max(40.0);
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), body_h),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.vertical(|ui| {
                    ui.add_space(2.0);
                    match panel {
                        PanelId::Health => {
                            if !is_input {
                                let (lvl, suit_col, suit_txt) = if self.state.hazard == Hazard::Hypoxia {
                                    (StatusLevel::Critical, t.critical, "O2 LOW")
                                } else if self.state.phase == Phase::ReEntry {
                                    (StatusLevel::Caution, t.caution, "FLUID+")
                                } else {
                                    (StatusLevel::Nominal, t.quad_health, "SUIT OK")
                                };

                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            draw_heart_icon(ui, 16.0 + pulse * 1.6, suit_col);
                                            ui.label(
                                                RichText::new(format!("{}", self.state.live_hr))
                                                    .family(FontFamily::Monospace)
                                                    .size(t.sz(26.0))
                                                    .color(t.text_main),
                                            );
                                            ui.label(
                                                RichText::new("bpm")
                                                    .family(FontFamily::Monospace)
                                                    .size(t.sz(10.5))
                                                    .color(t.text_muted),
                                            );
                                        });
                                        let (lo, hi) = self.state.phase.hr_bounds();
                                        let hr_anim = ctx.animate_value_with_time(
                                            Id::new("hr_smooth"),
                                            self.state.live_hr as f32,
                                            t.anim_secs,
                                        );
                                        draw_corridor_bar(ui, 108.0, hr_anim, lo as f32, hi as f32, suit_col, t);
                                        ui.add_space(2.0);
                                        status_chip(ui, lvl, suit_txt, suit_col, t);
                                    });

                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        // Mini Spacesuit Silhouette showing O2 Helmet, Cardio Core, CO2 Scrubber & Rad Shell
                                        draw_mini_spacesuit(
                                            ui,
                                            46.0,
                                            self.state.phase,
                                            self.state.hazard,
                                            pulse,
                                            t,
                                        );
                                    });
                                });
                            } else {
                                status_chip(ui, StatusLevel::Caution, "Meal & water log", t.accent_input, t);
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(format!("{}% meal · {} mL", self.state.meal_eaten_pct, self.state.water_ml))
                                        .family(FontFamily::Monospace)
                                        .size(t.sz(14.0))
                                        .color(t.text_main),
                                );
                            }
                        }
                        PanelId::Hub => {
                            if !is_input {
                                let dist_anim = ctx.animate_value_with_time(
                                    Id::new("dist_smooth"),
                                    self.state.distance_m as f32,
                                    t.anim_secs,
                                );
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(format!("{:.0}", dist_anim))
                                                    .family(FontFamily::Monospace)
                                                    .size(t.sz(26.0))
                                                    .color(t.text_main),
                                            );
                                            ui.label(
                                                RichText::new("m")
                                                    .family(FontFamily::Monospace)
                                                    .size(t.sz(11.5))
                                                    .color(t.text_muted),
                                            );
                                        });
                                        ui.label(
                                            RichText::new(self.state.phase.name())
                                                .size(t.sz(13.0))
                                                .color(t.text_body),
                                        );
                                        ui.label(
                                            RichText::new(ration.short)
                                                .size(t.sz(11.5))
                                                .color(t.quad_hub),
                                        );
                                    });
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        draw_compass_dial(ui, 44.0, 42.0_f32.to_radians(), t.quad_hub, t);
                                    });
                                });
                            } else {
                                status_chip(ui, StatusLevel::Caution, "Phase & sync", t.accent_input, t);
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(self.state.phase.name())
                                        .family(FontFamily::Monospace)
                                        .size(t.sz(15.0))
                                        .color(t.text_main),
                                );
                            }
                        }
                        PanelId::Tech => {
                            if !is_input {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        if self.state.online {
                                            status_chip(ui, StatusLevel::Nominal, "Online", t.nominal, t);
                                        } else {
                                            status_chip(ui, StatusLevel::Caution, "Offline", t.caution, t);
                                        }
                                        ui.add_space(2.0);
                                        if self.state.hazard != Hazard::Nominal {
                                            status_chip(ui, StatusLevel::Critical, "1 alert", t.critical, t);
                                        } else {
                                            ui.label(
                                                RichText::new("STD-3001 active")
                                                    .family(FontFamily::Monospace)
                                                    .size(t.sz(11.5))
                                                    .color(t.quad_comms),
                                            );
                                        }
                                    });
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        draw_signal_bars(ui, 30.0, self.state.online, t.quad_comms, t.caution, t);
                                    });
                                });
                            } else {
                                status_chip(ui, StatusLevel::Caution, "Blackout & A11y", t.accent_input, t);
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(if self.state.online { "LAN ON" } else { "LAN OFF" })
                                        .family(FontFamily::Monospace)
                                        .size(t.sz(14.0))
                                        .color(t.text_main),
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
                                // 3 visual mini-bars (Rad / CO2 / O2) readable at a glance
                                ui.horizontal(|ui| {
                                    gauge_pill(
                                        ui,
                                        ctx,
                                        "Rad",
                                        self.state.hazard.rad_val(),
                                        self.state.hazard.rad_ratio(),
                                        self.state.hazard == Hazard::Radiation,
                                        t,
                                    );
                                    ui.add_space(8.0);
                                    gauge_pill(
                                        ui,
                                        ctx,
                                        "CO2",
                                        self.state.hazard.co2_val(),
                                        self.state.hazard.co2_ratio(),
                                        self.state.hazard == Hazard::HighCo2,
                                        t,
                                    );
                                    ui.add_space(8.0);
                                    gauge_pill(
                                        ui,
                                        ctx,
                                        "O2",
                                        self.state.hazard.o2(),
                                        self.state.hazard.o2_ratio(),
                                        self.state.hazard == Hazard::Hypoxia,
                                        t,
                                    );
                                });
                            } else {
                                status_chip(ui, StatusLevel::Caution, "Test alerts", t.accent_input, t);
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(self.state.hazard.label())
                                        .size(t.sz(13.5))
                                        .color(t.text_main),
                                );
                            }
                        }
                    }
                });
            },
        );

        // Zone 3: Minimal Footer
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(footer_l)
                    .family(FontFamily::Monospace)
                    .size(t.sz(10.0))
                    .color(t.text_muted),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(footer_r)
                        .family(FontFamily::Monospace)
                        .size(t.sz(10.0))
                        .color(t.text_muted),
                );
            });
        });
    }

    fn render_detail_overlay(
        &mut self,
        ui: &mut Ui,
        ctx: &egui::Context,
        screen_rect: Rect,
        panel: PanelId,
        t: Tokens,
    ) {
        let is_input = self.state.mode == Mode::Input;
        let inner = screen_rect.shrink(16.0);
        let accent = if is_input {
            t.accent_input
        } else {
            t.quad_accent(panel)
        };

        let (kicker, title) = match (panel, is_input) {
            (PanelId::Health, false) => ("01 · Health & suit", "Suit vitals, body zones & ration"),
            (PanelId::Health, true) => ("01 · Edit mode", "Log meal & re-entry salt/water"),
            (PanelId::Hub, false) => ("02 · Hub & nav", "Range to habitat & signal link"),
            (PanelId::Hub, true) => ("02 · Edit mode", "Mission phase & hub sync"),
            (PanelId::Tech, false) => ("03 · Comms & display", "Local safety engine & display"),
            (PanelId::Tech, true) => ("03 · Edit mode", "Comms blackout & accessibility"),
            (PanelId::Hazard, false) => ("04 · Hazard & env", "Radiation, cabin gas & actions"),
            (PanelId::Hazard, true) => ("04 · Edit mode", "Test hazard alerts"),
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
                    let h_t = ctx.animate_bool_with_time(Id::new("ret_hover"), resp.hovered(), t.anim_secs);
                    let fill = lerp_color(t.raised_bg, t.raised_hover, h_t);
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
                    PanelId::Health => self.overlay_health(ui, ctx, is_input, t),
                    PanelId::Hub => self.overlay_hub(ui, ctx, is_input, t),
                    PanelId::Tech => self.overlay_tech(ui, ctx, is_input, t),
                    PanelId::Hazard => self.overlay_hazard(ui, ctx, is_input, t),
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
                    RichText::new("[C] Cycle mode · [1-4] Panels · [H] High-Visor")
                        .family(FontFamily::Monospace)
                        .size(t.sz(10.0))
                        .color(t.text_muted),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("Display: {}", self.state.a11y.summary()))
                            .family(FontFamily::Monospace)
                            .size(t.sz(10.0))
                            .color(t.text_muted),
                    );
                });
            });
        });
    }

    fn overlay_health(&mut self, ui: &mut Ui, ctx: &egui::Context, is_input: bool, t: Tokens) {
        let p = self.state.phase;
        let ration = self.state.compute_ration();
        if !is_input {
            ui.columns(4, |c| {
                stat_card(
                    &mut c[0],
                    "Heart rate",
                    &format!("{} bpm", self.state.live_hr),
                    t.quad_health,
                    &format!("Safe {}", p.hr_range()),
                    t,
                );
                stat_card(&mut c[1], "Blood pressure", p.bp(), t.text_main, "Cuffless optical", t);
                stat_card(
                    &mut c[2],
                    "Calorie deficit",
                    &format!("{} kcal", self.state.calorie_deficit),
                    t.caution,
                    &format!("{:.1} kcal/min", p.kcal_rate()),
                    t,
                );
                stat_card(
                    &mut c[3],
                    "Hydration loss",
                    &format!("{} mL", self.state.sweat_deficit_ml),
                    t.quad_hub,
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
                        100 => "Full",
                        0 => "Skipped",
                        _ => "Partial",
                    };
                    if glove_button(
                        &mut c[idx],
                        ctx,
                        ("meal_btn", idx),
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
                    ctx,
                    "water_btn",
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
                    ctx,
                    "nacl_btn",
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

    fn overlay_hub(&mut self, ui: &mut Ui, ctx: &egui::Context, is_input: bool, t: Tokens) {
        let p = self.state.phase;
        if !is_input {
            ui.columns(4, |c| {
                stat_card(&mut c[0], "Hub range", &format!("{} m", self.state.distance_m), t.quad_hub, "Bearing 042° NE", t);
                stat_card(&mut c[1], "Flight phase", p.name(), t.text_main, &format!("Safe {}", p.hr_range()), t);
                stat_card(&mut c[2], "Earth lag", "18m 42s", t.caution, "Suit runs locally", t);
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
                    if glove_button(&mut c[i], ctx, ("phase_btn", i), ph.name(), ph.hr_range(), self.state.phase == *ph, t).clicked() {
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
                    ctx,
                    "sync_btn",
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
                if glove_button(&mut c[1], ctx, "wp_btn", "Set airlock waypoint", "145 m range", false, t).clicked() {
                    self.state.distance_m = 145;
                }
            });
        }
    }

    fn overlay_tech(&mut self, ui: &mut Ui, ctx: &egui::Context, is_input: bool, t: Tokens) {
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
                stat_card(&mut c[3], "Medical DB", "64 MB local", t.quad_comms, "NASA-STD-3001", t);
            });
            ui.add_space(12.0);
            rag_box(
                ui,
                "Offline crater safety & display profile",
                None,
                &format!(
                    "When crater walls block surface radio, your suit monitors NASA-STD-3001 thresholds locally every second and stores packets until reconnect. Active display profile: {}.",
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
                    ctx,
                    "a11y_hv",
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
                    ctx,
                    "a11y_lg",
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
                    ctx,
                    "a11y_cb",
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
                    ctx,
                    "a11y_gl",
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
                ctx,
                "a11y_rm",
                "Reduced motion (static vitals)",
                if self.state.a11y.reduced_motion { "ON" } else { "OFF" },
                self.state.a11y.reduced_motion,
                t,
            )
            .clicked()
            {
                self.state.a11y.reduced_motion = !self.state.a11y.reduced_motion;
            }

            ui.add_space(12.0);
            ui.label(RichText::new("LAN blackout test & queue").size(t.sz(12.5)).color(t.text_body));
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
            if glove_button(ui, ctx, "lan_btn", label, right, !self.state.online, t).clicked() {
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
                t.quad_comms,
                "Saves 1 frame/s offline; flushes on reconnect.",
                t,
            );
        }
    }

    fn overlay_hazard(&mut self, ui: &mut Ui, ctx: &egui::Context, is_input: bool, t: Tokens) {
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
            for (row, pair) in Hazard::ALL.chunks(2).enumerate() {
                ui.columns(2, |c| {
                    for (i, hk) in pair.iter().enumerate() {
                        let sub = if *hk == Hazard::Nominal { "Clear alert" } else { "Trigger alert" };
                        if glove_button(&mut c[i], ctx, ("haz_btn", row * 2 + i), hk.label(), sub, self.state.hazard == *hk, t).clicked() {
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

// ───────────────────────── Visual & Smooth Craft Primitives ─────────────────────────
fn fit_16_by_9(bounds: Rect) -> Rect {
    let target = 16.0_f32 / 9.0_f32;
    let (w, h) = if bounds.width() / bounds.height() > target {
        (bounds.height() * target, bounds.height())
    } else {
        (bounds.width(), bounds.width() / target)
    };
    Rect::from_center_size(bounds.center(), Vec2::new(w, h))
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| -> u8 { (x as f32 + (y as f32 - x as f32) * t).round() as u8 };
    Color32::from_rgba_unmultiplied(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()), mix(a.a(), b.a()))
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
        ui.label(RichText::new(label).size(t.sz(13.0)).color(color));
    });
}

fn draw_mini_spacesuit(
    ui: &mut Ui,
    size: f32,
    phase: Phase,
    hazard: Hazard,
    pulse: f32,
    t: Tokens,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(size, size * 1.15), Sense::hover());
    let p = ui.painter();
    let cx = rect.center().x;
    let top = rect.top() + 3.0;

    let helmet_col = if hazard == Hazard::Hypoxia { t.critical } else { t.quad_hub };
    let torso_col = if hazard == Hazard::Hypoxia {
        t.critical
    } else if phase == Phase::ReEntry {
        t.caution
    } else {
        t.quad_health
    };
    let pack_col = if hazard == Hazard::HighCo2 { t.caution } else { t.hairline_strong };
    let shield_col = if hazard == Hazard::Radiation { t.critical } else { t.hairline };

    // Optional radiation outer arc indicators
    if hazard == Hazard::Radiation {
        p.circle_stroke(rect.center(), size * 0.48, Stroke::new(1.5_f32, t.critical));
    } else {
        p.circle_stroke(rect.center(), size * 0.48, Stroke::new(1.0_f32, shield_col.gamma_multiply(0.35)));
    }

    // PLSS Backpack (CO2 scrubber indicator on side)
    let pack = Rect::from_min_size(Pos2::new(cx - 11.0, top + 14.0), Vec2::new(4.0, 14.0));
    p.rect_filled(pack, 1.5, pack_col);

    // Helmet + Visor (O2 indicator)
    let head_c = Pos2::new(cx, top + 7.5);
    p.circle(head_c, 6.5, t.screen_bg, Stroke::new(1.5_f32, helmet_col));
    p.rect_filled(Rect::from_center_size(Pos2::new(cx + 1.0, head_c.y), Vec2::new(6.0, 4.0)), 1.5, helmet_col);

    // Torso (Suit integrity + live pulsing core)
    let torso = Rect::from_min_size(Pos2::new(cx - 6.5, top + 15.0), Vec2::new(13.0, 15.0));
    p.rect(torso, 3.0, torso_col.gamma_multiply(0.18), Stroke::new(1.5_f32, torso_col));
    p.circle_filled(torso.center(), 2.2 + pulse * 0.9, torso_col);

    // Arms
    p.line_segment([Pos2::new(torso.left(), torso.top() + 2.0), Pos2::new(torso.left() - 4.5, torso.top() + 11.0)], Stroke::new(2.0_f32, torso_col));
    p.line_segment([Pos2::new(torso.right(), torso.top() + 2.0), Pos2::new(torso.right() + 4.5, torso.top() + 11.0)], Stroke::new(2.0_f32, torso_col));

    // Legs
    p.line_segment([Pos2::new(cx - 3.0, torso.bottom()), Pos2::new(cx - 4.0, torso.bottom() + 12.0)], Stroke::new(2.2_f32, torso_col));
    p.line_segment([Pos2::new(cx + 3.0, torso.bottom()), Pos2::new(cx + 4.0, torso.bottom() + 12.0)], Stroke::new(2.2_f32, torso_col));
}

fn draw_signal_bars(ui: &mut Ui, size: f32, online: bool, on_col: Color32, off_col: Color32, t: Tokens) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let p = ui.painter();
    let active_bars = if online { 4 } else { 1 };
    let col = if online { on_col } else { off_col };
    let w = 4.5_f32;
    let gap = 2.5_f32;
    let start_x = rect.left() + 2.0;
    for i in 0..4 {
        let h = 6.0 + (i as f32) * 5.0;
        let x = start_x + (i as f32) * (w + gap);
        let bar = Rect::from_min_size(Pos2::new(x, rect.bottom() - 3.0 - h), Vec2::new(w, h));
        let fill = if i < active_bars { col } else { t.hairline };
        p.rect_filled(bar, 1.0, fill);
    }
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

    let safe_left = track.left() + width * 0.20;
    let safe_w = width * 0.60;
    let safe_rect = Rect::from_min_size(Pos2::new(safe_left, track.top()), Vec2::new(safe_w, 4.0));
    p.rect_filled(safe_rect, 2.0, t.hairline_strong);

    let span = (hi - lo).max(1.0);
    let norm = ((val - lo) / span).clamp(0.0, 1.0);
    let x = safe_left + safe_w * norm;
    p.circle_filled(Pos2::new(x, track.center().y), 3.5, marker_col);
}

fn gauge_pill(
    ui: &mut Ui,
    ctx: &egui::Context,
    name: &str,
    val: &str,
    target_ratio: f32,
    alerted: bool,
    t: Tokens,
) {
    let col = if alerted { t.critical } else { t.text_main };
    let bar_col = if alerted { t.critical } else { t.nominal };
    let fill_ratio = ctx.animate_value_with_time(Id::new(("haz_gauge", name)), target_ratio.clamp(0.05, 1.0), t.anim_secs);
    ui.vertical(|ui| {
        ui.label(
            RichText::new(format!("{name} {val}"))
                .family(FontFamily::Monospace)
                .size(t.sz(11.0))
                .color(col),
        );
        let (rect, _) = ui.allocate_exact_size(Vec2::new(62.0, 4.0), Sense::hover());
        ui.painter().rect_filled(rect, 2.0, t.hairline);
        let fill = Rect::from_min_size(rect.min, Vec2::new(rect.width() * fill_ratio, rect.height()));
        ui.painter().rect_filled(fill, 2.0, bar_col);
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

fn glove_button(
    ui: &mut Ui,
    ctx: &egui::Context,
    anim_key: impl std::hash::Hash,
    left: &str,
    right: &str,
    selected: bool,
    t: Tokens,
) -> egui::Response {
    let desired = Vec2::new(ui.available_width(), t.button_h);
    let (rect, resp) = ui.allocate_exact_size(desired, Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    let h_t = ctx.animate_bool_with_time(Id::new(anim_key), resp.hovered(), t.anim_secs);
    let (bg, border, text_col) = if selected {
        (t.card_input_hover, t.accent_input, t.accent_input)
    } else {
        (lerp_color(t.raised_bg, t.raised_hover, h_t), t.hairline_strong, t.text_main)
    };

    let p = ui.painter();
    p.rect(rect, 8.0, bg, Stroke::new(t.stroke_w, border));
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
        assert_eq!(a11y_t.anim_secs, 0.0);
        assert!(a11y_t.sz(12.0) > std_t.sz(12.0));
    }

    #[test]
    fn each_quadrant_has_distinct_color_signature() {
        let t = Tokens::resolve(AccessibilityConfig::default());
        let cols = [
            t.quad_accent(PanelId::Health),
            t.quad_accent(PanelId::Hub),
            t.quad_accent(PanelId::Tech),
            t.quad_accent(PanelId::Hazard),
        ];
        for i in 0..cols.len() {
            for j in (i + 1)..cols.len() {
                assert_ne!(cols[i], cols[j]);
            }
        }
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
