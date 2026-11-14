use eframe::egui::{
    self, Align, Align2, Color32, CursorIcon, FontData, FontDefinitions, FontFamily, FontId,
    Frame, Id, Layout, Margin, Order, Pos2, Rect, RichText, Rounding, Sense, Shape, Stroke, Ui,
    Vec2,
};
use std::time::{Duration, Instant};

// ───────────────────────── Layer 1: Primitive Palettes ─────────────────────────
mod palette {
    use eframe::egui::Color32;

    pub const VOID_950: Color32 = Color32::from_rgb(5, 7, 11);
    pub const SCREEN_900: Color32 = Color32::from_rgb(10, 13, 19);
    pub const VISOR_950: Color32 = Color32::from_rgb(0, 0, 0);
    pub const VISOR_900: Color32 = Color32::from_rgb(4, 6, 9);

    pub const SLATE_850: Color32 = Color32::from_rgb(18, 23, 33);
    pub const SLATE_800: Color32 = Color32::from_rgb(26, 33, 47);
    pub const SLATE_700: Color32 = Color32::from_rgb(46, 58, 78);
    pub const SLATE_600: Color32 = Color32::from_rgb(70, 86, 112);

    // Full-Card Subsystem Surfaces
    pub const HEALTH_BG: Color32 = Color32::from_rgb(11, 36, 29);
    pub const HEALTH_HOVER: Color32 = Color32::from_rgb(16, 51, 41);
    pub const HEALTH_BORDER: Color32 = Color32::from_rgb(29, 98, 73);
    pub const HEALTH_ACCENT: Color32 = Color32::from_rgb(52, 211, 153);

    pub const HUB_BG: Color32 = Color32::from_rgb(10, 30, 46);
    pub const HUB_HOVER: Color32 = Color32::from_rgb(15, 44, 67);
    pub const HUB_BORDER: Color32 = Color32::from_rgb(31, 92, 132);
    pub const HUB_ACCENT: Color32 = Color32::from_rgb(56, 189, 248);

    pub const COMMS_BG: Color32 = Color32::from_rgb(20, 25, 52);
    pub const COMMS_HOVER: Color32 = Color32::from_rgb(29, 36, 74);
    pub const COMMS_BORDER: Color32 = Color32::from_rgb(66, 82, 154);
    pub const COMMS_ACCENT: Color32 = Color32::from_rgb(129, 140, 248);

    pub const HAZARD_BG: Color32 = Color32::from_rgb(35, 26, 11);
    pub const HAZARD_HOVER: Color32 = Color32::from_rgb(49, 36, 15);
    pub const HAZARD_BORDER: Color32 = Color32::from_rgb(118, 82, 24);
    pub const HAZARD_ACCENT: Color32 = Color32::from_rgb(245, 158, 11);

    // Edit Mode Warm Tinted Surfaces
    pub const EDIT_BG: Color32 = Color32::from_rgb(36, 27, 13);
    pub const EDIT_HOVER: Color32 = Color32::from_rgb(50, 38, 18);
    pub const EDIT_BORDER: Color32 = Color32::from_rgb(196, 132, 24);

    // Critical Alert Surfaces
    pub const ALERT_BG: Color32 = Color32::from_rgb(48, 15, 20);
    pub const ALERT_HOVER: Color32 = Color32::from_rgb(66, 20, 27);

    // Text Ramp
    pub const TEXT_WHITE: Color32 = Color32::from_rgb(246, 248, 252);
    pub const TEXT_BODY: Color32 = Color32::from_rgb(209, 217, 230);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(148, 160, 181);

    // Semantics
    pub const NOMINAL_GREEN: Color32 = Color32::from_rgb(74, 222, 128);
    pub const CAUTION_AMBER: Color32 = Color32::from_rgb(251, 191, 36);
    pub const CRITICAL_RED: Color32 = Color32::from_rgb(248, 113, 113);

    // Colorblind Palettes
    pub const DEUT_OK: Color32 = Color32::from_rgb(0, 186, 156);
    pub const DEUT_WARN: Color32 = Color32::from_rgb(230, 159, 0);
    pub const DEUT_CRIT: Color32 = Color32::from_rgb(244, 114, 182);
    pub const DEUT_BLUE: Color32 = Color32::from_rgb(86, 180, 233);

    pub const TRIT_OK: Color32 = Color32::from_rgb(56, 189, 248);
    pub const TRIT_WARN: Color32 = Color32::from_rgb(251, 146, 60);
    pub const TRIT_CRIT: Color32 = Color32::from_rgb(239, 68, 68);

    pub const MONO_BRIGHT: Color32 = Color32::from_rgb(248, 250, 252);
    pub const MONO_MID: Color32 = Color32::from_rgb(186, 196, 210);
}

// ───────────────────────── Layer 2 & 3: Accessibility + Responsive Tokens ─────────────────────────
#[derive(Clone, Copy, PartialEq, Eq)]
enum ColorTheme {
    FullColorCards,
    NightDark,
    HighVisorAaa,
}

impl ColorTheme {
    const ALL: [ColorTheme; 3] = [Self::FullColorCards, Self::NightDark, Self::HighVisorAaa];

    fn label(self) -> &'static str {
        match self {
            Self::FullColorCards => "Color Cards",
            Self::NightDark => "Night Dark",
            Self::HighVisorAaa => "Sun Visor",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ColorVision {
    Standard,
    RedGreenSafe,
    BlueYellowSafe,
    Monochrome,
}

impl ColorVision {
    const ALL: [ColorVision; 4] = [
        Self::Standard,
        Self::RedGreenSafe,
        Self::BlueYellowSafe,
        Self::Monochrome,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Standard => "Standard",
            Self::RedGreenSafe => "Red-Green Safe",
            Self::BlueYellowSafe => "Blue-Yellow Safe",
            Self::Monochrome => "Monochrome",
        }
    }

    fn short(self) -> &'static str {
        match self {
            Self::Standard => "Std",
            Self::RedGreenSafe => "RG",
            Self::BlueYellowSafe => "BY",
            Self::Monochrome => "Mono",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TextScalePreset {
    Compact,
    Normal,
    Large,
}

impl TextScalePreset {
    const ALL: [TextScalePreset; 3] = [Self::Compact, Self::Normal, Self::Large];

    fn factor(self) -> f32 {
        match self {
            Self::Compact => 0.92,
            Self::Normal => 1.0,
            Self::Large => 1.12,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Compact => "Compact",
            Self::Normal => "Default",
            Self::Large => "Large",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
struct AccessibilityConfig {
    theme: ColorTheme,
    vision: ColorVision,
    brightness_pct: u16,
    text_scale: TextScalePreset,
    glove_targets: bool,
    reduced_motion: bool,
}

impl Default for AccessibilityConfig {
    fn default() -> Self {
        Self {
            theme: ColorTheme::FullColorCards,
            vision: ColorVision::Standard,
            brightness_pct: 100,
            text_scale: TextScalePreset::Normal,
            glove_targets: false,
            reduced_motion: false,
        }
    }
}

impl AccessibilityConfig {
    const BRIGHTNESS_MIN: u16 = 25;
    const BRIGHTNESS_MAX: u16 = 175;

    fn summary(self) -> String {
        format!(
            "{} · {}% · {}",
            self.theme.label(),
            self.brightness_pct,
            self.vision.short()
        )
    }
}

#[derive(Clone, Copy)]
struct CardColors {
    bg: Color32,
    hover: Color32,
    border: Color32,
    accent: Color32,
}

#[derive(Clone, Copy)]
struct Tokens {
    outer_bg: Color32,
    screen_bg: Color32,
    raised_bg: Color32,
    raised_hover: Color32,
    hairline: Color32,
    hairline_strong: Color32,
    accent_view: Color32,
    accent_input: Color32,
    nominal: Color32,
    caution: Color32,
    critical: Color32,
    alert_card: CardColors,
    edit_card: CardColors,
    text_main: Color32,
    text_body: Color32,
    text_muted: Color32,
    ui_scale: f32,
    text_mult: f32,
    stroke_w: f32,
    gap: f32,
    button_h: f32,
    anim_secs: f32,
    brightness: f32,
    theme: ColorTheme,
    vision: ColorVision,
}

impl Tokens {
    fn resolve(cfg: AccessibilityConfig, screen_rect: Rect) -> Self {
        let ui_scale = (screen_rect.height() / 504.0).clamp(0.68, 2.40);
        let text_mult = ui_scale * cfg.text_scale.factor();
        let b = (cfg.brightness_pct as f32) / 100.0;
        let high = cfg.theme == ColorTheme::HighVisorAaa;

        let (nominal, caution, critical, accent_view) = match cfg.vision {
            ColorVision::Standard => (
                palette::NOMINAL_GREEN,
                palette::CAUTION_AMBER,
                palette::CRITICAL_RED,
                palette::HUB_ACCENT,
            ),
            ColorVision::RedGreenSafe => (
                palette::DEUT_OK,
                palette::DEUT_WARN,
                palette::DEUT_CRIT,
                palette::DEUT_BLUE,
            ),
            ColorVision::BlueYellowSafe => (
                palette::TRIT_OK,
                palette::TRIT_WARN,
                palette::TRIT_CRIT,
                palette::HEALTH_ACCENT,
            ),
            ColorVision::Monochrome => (
                palette::MONO_BRIGHT,
                palette::MONO_MID,
                palette::MONO_BRIGHT,
                palette::MONO_BRIGHT,
            ),
        };

        let dim = |c: Color32| scale_brightness(c, b);
        let glove_mult = if cfg.glove_targets { 1.12 } else { 1.0 };

        Self {
            outer_bg: if high { palette::VISOR_950 } else { dim(palette::VOID_950) },
            screen_bg: if high { palette::VISOR_900 } else { dim(palette::SCREEN_900) },
            raised_bg: dim(if high { palette::SLATE_800 } else { palette::SLATE_850 }),
            raised_hover: dim(palette::SLATE_800),
            hairline: dim(if high { palette::SLATE_600 } else { palette::SLATE_700 }),
            hairline_strong: dim(if high { palette::TEXT_WHITE } else { palette::SLATE_600 }),
            accent_view: dim(accent_view),
            accent_input: dim(caution),
            nominal: dim(nominal),
            caution: dim(caution),
            critical: dim(critical),
            alert_card: CardColors {
                bg: dim(palette::ALERT_BG),
                hover: dim(palette::ALERT_HOVER),
                border: dim(critical),
                accent: dim(critical),
            },
            edit_card: CardColors {
                bg: dim(palette::EDIT_BG),
                hover: dim(palette::EDIT_HOVER),
                border: dim(if high { caution } else { palette::EDIT_BORDER }),
                accent: dim(caution),
            },
            text_main: dim(if high { Color32::WHITE } else { palette::TEXT_WHITE }),
            text_body: dim(if high { Color32::WHITE } else { palette::TEXT_BODY }),
            text_muted: dim(if high { palette::TEXT_BODY } else { palette::TEXT_MUTED }),
            ui_scale,
            text_mult,
            stroke_w: ((if high { 1.8_f32 } else { 1.2_f32 }) * ui_scale.clamp(0.85, 1.4))
                .round()
                .max(1.0),
            gap: (11.0 * ui_scale).round().clamp(7.0, 24.0),
            button_h: (36.0 * ui_scale * glove_mult).round().clamp(26.0, 68.0),
            anim_secs: if cfg.reduced_motion { 0.0 } else { 0.08 },
            brightness: b,
            theme: cfg.theme,
            vision: cfg.vision,
        }
    }

    fn sp(&self, base: f32) -> f32 {
        (base * self.ui_scale).round()
    }

    fn sz(&self, base: f32) -> f32 {
        (base * self.text_mult).clamp(9.0, 64.0)
    }

    fn card_colors(&self, panel: PanelId) -> CardColors {
        let b = self.brightness;
        let d = |c: Color32| scale_brightness(c, b);

        let accent = match (panel, self.vision) {
            (_, ColorVision::Monochrome) => d(palette::MONO_BRIGHT),
            (PanelId::Health, ColorVision::RedGreenSafe) => d(palette::DEUT_OK),
            (PanelId::Hub, ColorVision::RedGreenSafe) => d(palette::DEUT_BLUE),
            (PanelId::Tech, ColorVision::RedGreenSafe) => d(palette::COMMS_ACCENT),
            (PanelId::Hazard, ColorVision::RedGreenSafe) => d(palette::DEUT_WARN),
            (PanelId::Health, ColorVision::BlueYellowSafe) => d(palette::TRIT_OK),
            (PanelId::Hub, ColorVision::BlueYellowSafe) => d(palette::HEALTH_ACCENT),
            (PanelId::Tech, ColorVision::BlueYellowSafe) => d(palette::TRIT_WARN),
            (PanelId::Hazard, ColorVision::BlueYellowSafe) => d(palette::TRIT_CRIT),
            (PanelId::Health, _) => d(palette::HEALTH_ACCENT),
            (PanelId::Hub, _) => d(palette::HUB_ACCENT),
            (PanelId::Tech, _) => d(palette::COMMS_ACCENT),
            (PanelId::Hazard, _) => d(palette::HAZARD_ACCENT),
        };

        match self.theme {
            ColorTheme::FullColorCards => {
                if self.vision == ColorVision::Monochrome {
                    return CardColors {
                        bg: d(palette::SLATE_850),
                        hover: d(palette::SLATE_800),
                        border: d(palette::SLATE_600),
                        accent,
                    };
                }
                match panel {
                    PanelId::Health => CardColors {
                        bg: d(palette::HEALTH_BG),
                        hover: d(palette::HEALTH_HOVER),
                        border: d(palette::HEALTH_BORDER),
                        accent,
                    },
                    PanelId::Hub => CardColors {
                        bg: d(palette::HUB_BG),
                        hover: d(palette::HUB_HOVER),
                        border: d(palette::HUB_BORDER),
                        accent,
                    },
                    PanelId::Tech => CardColors {
                        bg: d(palette::COMMS_BG),
                        hover: d(palette::COMMS_HOVER),
                        border: d(palette::COMMS_BORDER),
                        accent,
                    },
                    PanelId::Hazard => CardColors {
                        bg: d(palette::HAZARD_BG),
                        hover: d(palette::HAZARD_HOVER),
                        border: d(palette::HAZARD_BORDER),
                        accent,
                    },
                }
            }
            ColorTheme::NightDark => CardColors {
                bg: d(palette::SLATE_850),
                hover: d(palette::SLATE_800),
                border: accent.gamma_multiply(0.45),
                accent,
            },
            ColorTheme::HighVisorAaa => CardColors {
                bg: palette::VISOR_900,
                hover: d(palette::SLATE_850),
                border: accent,
                accent,
            },
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StatusLevel {
    Nominal,
    Caution,
    Critical,
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 540.0])
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

// ───────────────────────── Suit Condition & Repair Domain ─────────────────────────
#[derive(Clone, Copy, PartialEq, Eq)]
enum SuitFault {
    Nominal,
    GloveTear,
    ScrubberDrift,
    CoreBreach,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Repairability {
    Intact,
    FieldRepairable,
    AirlockRequired,
}

struct SuitZoneStatus {
    zone: &'static str,
    integrity_pct: u8,
    repairability: Repairability,
    action: &'static str,
}

impl SuitFault {
    const ALL: [SuitFault; 4] = [
        Self::Nominal,
        Self::GloveTear,
        Self::ScrubberDrift,
        Self::CoreBreach,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Nominal => "Suit 100%",
            Self::GloveTear => "Glove seam tear",
            Self::ScrubberDrift => "Scrubber clog",
            Self::CoreBreach => "Visor / hull crack",
        }
    }

    fn sub(self) -> &'static str {
        match self {
            Self::Nominal => "Nominal",
            Self::GloveTear => "Repairable",
            Self::ScrubberDrift => "Repairable",
            Self::CoreBreach => "Unrepairable",
        }
    }
}

// ───────────────────────── Mission Telemetry State ─────────────────────────
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
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NavWaypoint {
    Habitat,
    Airlock,
}

impl NavWaypoint {
    fn label(self) -> &'static str {
        match self {
            Self::Habitat => "Habitat",
            Self::Airlock => "Airlock",
        }
    }

    fn bearing_deg(self) -> f32 {
        match self {
            Self::Habitat => 42.0,
            Self::Airlock => 312.0,
        }
    }

    fn cardinal(self) -> &'static str {
        match self {
            Self::Habitat => "NE",
            Self::Airlock => "NW",
        }
    }
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
    suit_fault: SuitFault,
    waypoint: NavWaypoint,
    distance_m: u32,
    compass_heading_deg: f32,
    calorie_deficit: u32,
    sweat_deficit_ml: u32,
    live_hr: i32,
    active_panel: Option<PanelId>,
    a11y: AccessibilityConfig,
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
            suit_fault: SuitFault::Nominal,
            waypoint: NavWaypoint::Habitat,
            distance_m: 222,
            compass_heading_deg: 42.0,
            calorie_deficit: 120,
            sweat_deficit_ml: 85,
            live_hr: 88,
            active_panel: None,
            a11y: AccessibilityConfig::default(),
        }
    }
}

impl BiomonState {
    fn relative_bearing_deg(&self) -> f32 {
        (self.waypoint.bearing_deg() - self.compass_heading_deg + 360.0) % 360.0
    }

    fn set_waypoint(&mut self, wp: NavWaypoint) {
        self.waypoint = wp;
        self.distance_m = match wp {
            NavWaypoint::Habitat => 222,
            NavWaypoint::Airlock => 145,
        };
    }

    fn suit_zones(&self) -> [SuitZoneStatus; 5] {
        let helmet = if self.hazard == Hazard::Hypoxia {
            SuitZoneStatus {
                zone: "Helmet & O2",
                integrity_pct: 58,
                repairability: Repairability::FieldRepairable,
                action: "Field repair: switch to Backup O2 Pack",
            }
        } else if self.suit_fault == SuitFault::CoreBreach {
            SuitZoneStatus {
                zone: "Helmet & O2",
                integrity_pct: 32,
                repairability: Repairability::AirlockRequired,
                action: "NOT repairable in EVA — return to airlock",
            }
        } else {
            SuitZoneStatus {
                zone: "Helmet & O2",
                integrity_pct: 100,
                repairability: Repairability::Intact,
                action: "4.3 psi seal intact",
            }
        };

        let torso = if self.suit_fault == SuitFault::CoreBreach {
            SuitZoneStatus {
                zone: "Torso shell",
                integrity_pct: 28,
                repairability: Repairability::AirlockRequired,
                action: "Pressure bladder breach — abort EVA",
            }
        } else {
            SuitZoneStatus {
                zone: "Torso shell",
                integrity_pct: 100,
                repairability: Repairability::Intact,
                action: "Cooling loop & hull OK",
            }
        };

        let scrubber = if self.hazard == Hazard::HighCo2 || self.suit_fault == SuitFault::ScrubberDrift {
            SuitZoneStatus {
                zone: "PLSS scrubber",
                integrity_pct: 52,
                repairability: Repairability::FieldRepairable,
                action: "Field repair: swap spare LiOH cartridge",
            }
        } else {
            SuitZoneStatus {
                zone: "PLSS scrubber",
                integrity_pct: 98,
                repairability: Repairability::Intact,
                action: "CO2 scrub flow nominal",
            }
        };

        let gloves = if self.suit_fault == SuitFault::GloveTear {
            SuitZoneStatus {
                zone: "Gloves & seams",
                integrity_pct: 64,
                repairability: Repairability::FieldRepairable,
                action: "Field repair: Kapton patch from thigh kit",
            }
        } else {
            SuitZoneStatus {
                zone: "Gloves & seams",
                integrity_pct: 100,
                repairability: Repairability::Intact,
                action: "Wrist bearing seals tight",
            }
        };

        let shield = if self.hazard == Hazard::Radiation {
            SuitZoneStatus {
                zone: "Rad shielding",
                integrity_pct: 45,
                repairability: Repairability::AirlockRequired,
                action: "Shelter needed — unrepairable outside",
            }
        } else {
            SuitZoneStatus {
                zone: "Rad shielding",
                integrity_pct: 100,
                repairability: Repairability::Intact,
                action: "Thermal & dust shield intact",
            }
        };

        [helmet, torso, scrubber, gloves, shield]
    }

    fn overall_suit_repairability(&self) -> Repairability {
        let zones = self.suit_zones();
        if zones.iter().any(|z| z.repairability == Repairability::AirlockRequired) {
            Repairability::AirlockRequired
        } else if zones.iter().any(|z| z.repairability == Repairability::FieldRepairable) {
            Repairability::FieldRepairable
        } else {
            Repairability::Intact
        }
    }

    fn apply_field_repair(&mut self) {
        if self.suit_fault == SuitFault::GloveTear || self.suit_fault == SuitFault::ScrubberDrift {
            self.suit_fault = SuitFault::Nominal;
        }
        if self.hazard == Hazard::HighCo2 || self.hazard == Hazard::Hypoxia {
            self.hazard = Hazard::Nominal;
        }
    }

    fn ration_short(&self) -> &'static str {
        if self.phase == Phase::ReEntry {
            "1.15 L + 9x NaCl"
        } else if self.hazard == Hazard::HighCo2 {
            "Low-Na+ salmon"
        } else if self.meal_eaten_pct < 100 || self.phase == Phase::Eva {
            "High-K+ fajita"
        } else {
            "Balanced #4"
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
        install_aerospace_fonts(&cc.egui_ctx);
        let app = Self {
            state: BiomonState::default(),
            last_tick: Instant::now(),
            prng: 0xAE61_5B10_2026_1114,
        };
        let init_t = Tokens::resolve(
            app.state.a11y,
            Rect::from_min_size(Pos2::ZERO, Vec2::new(960.0, 540.0)),
        );
        app.apply_style(&cc.egui_ctx, init_t);
        app
    }

    fn apply_style(&self, ctx: &egui::Context, t: Tokens) {
        ctx.style_mut(|s| {
            s.spacing.item_spacing = Vec2::splat(t.sp(6.0).max(4.0));
            s.spacing.button_padding = Vec2::new(t.sp(10.0), t.sp(6.0));
            s.interaction.selectable_labels = false;
        });
        let mut v = egui::Visuals::dark();
        v.panel_fill = t.outer_bg;
        v.window_fill = t.screen_bg;
        v.override_text_color = Some(t.text_main);
        let r = Rounding::same(t.sp(8.0).max(5.0));
        v.widgets.noninteractive.rounding = r;
        v.widgets.inactive.rounding = r;
        v.widgets.hovered.rounding = r;
        v.widgets.active.rounding = r;
        ctx.set_visuals(v);
    }

    fn next_rand(&mut self) -> u64 {
        self.prng = self
            .prng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.prng >> 32
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

        // Realistic gentle wrist heading drift (live working compass)
        let heading_step = if self.state.phase == Phase::Eva {
            ((self.next_rand() % 7) as f32) - 3.0
        } else {
            ((self.next_rand() % 3) as f32) - 1.0
        };
        self.state.compass_heading_deg = (self.state.compass_heading_deg + heading_step + 360.0) % 360.0;

        if self.state.phase == Phase::Eva && (self.next_rand() % 2 == 1) {
            self.state.distance_m += 1;
        }

        if !self.state.online {
            self.state.buffered_packets += 1;
        }
    }
}

fn install_aerospace_fonts(ctx: &egui::Context) {
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

fn heading_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("heading".into()))
}

impl eframe::App for BiomonApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.tick_1hz();
        // Clean 250ms cadence without stifling continuous 30fps loop; pointer clicks repaint immediately
        ctx.request_repaint_after(Duration::from_millis(250));

        egui::CentralPanel::default()
            .frame(Frame::none().fill(palette::VOID_950).inner_margin(Margin::same(0.0)))
            .show(ctx, |ui| {
                let avail = ui.available_rect_before_wrap();
                let margined = avail.shrink2(Vec2::new(avail.width() * 0.015, avail.height() * 0.015));
                let screen_rect = fit_16_by_9(margined);

                let t = Tokens::resolve(self.state.a11y, screen_rect);
                self.apply_style(ctx, t);

                let painter = ui.painter();
                painter.rect_filled(avail, 0.0, t.outer_bg);
                painter.rect_filled(screen_rect, t.sp(14.0), t.screen_bg);
                painter.rect_stroke(screen_rect, t.sp(14.0), Stroke::new(t.stroke_w, t.hairline_strong));

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
        let inner_r = (screen_rect.height() * 0.17).max(48.0);
        let ring_r = inner_r + t.sp(7.0);

        let pointer_in_cycle = ctx
            .input(|i| i.pointer.hover_pos())
            .is_some_and(|pos| pos.distance(center) <= ring_r);

        let gap = t.gap;
        let grid_rect = screen_rect.shrink(gap);
        let qw = (grid_rect.width() - gap) / 2.0;
        let qh = (grid_rect.height() - gap) / 2.0;

        let outer_pad = t.sp(14.0).max(10.0);
        let v_pad = t.sp(10.0).max(7.0);
        let clear_x = (ring_r * 0.85).max(outer_pad);
        let clear_y = (ring_r * 0.30).max(v_pad);

        let rects = [
            (
                PanelId::Health,
                Rect::from_min_size(grid_rect.min, Vec2::new(qw, qh)),
                Margin {
                    left: outer_pad,
                    right: clear_x,
                    top: v_pad,
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
                    right: outer_pad,
                    top: v_pad,
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
                    left: outer_pad,
                    right: clear_x,
                    top: clear_y,
                    bottom: v_pad,
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
                    right: outer_pad,
                    top: clear_y,
                    bottom: v_pad,
                },
            ),
        ];

        let card_r = t.sp(12.0).max(8.0);
        let suit_rep = self.state.overall_suit_repairability();
        let is_edit = self.state.mode == Mode::Input;

        for (panel_id, q_rect, pad) in rects {
            let is_alert = match panel_id {
                PanelId::Health => {
                    self.state.hazard == Hazard::Hypoxia || suit_rep == Repairability::AirlockRequired
                }
                PanelId::Hazard => self.state.hazard != Hazard::Nominal,
                _ => false,
            };

            let resp = ui.interact(q_rect, ui.id().with(("quad_btn", panel_id as u8)), Sense::click());
            let hovered = resp.hovered() && !pointer_in_cycle;
            if hovered {
                ctx.set_cursor_icon(CursorIcon::PointingHand);
            }

            let hover_t = ctx.animate_bool_with_time(Id::new(("quad_hover", panel_id as u8)), hovered, t.anim_secs);
            let base_c = t.card_colors(panel_id);
            let target_c = if is_alert {
                t.alert_card
            } else if is_edit {
                CardColors {
                    bg: lerp_color(base_c.bg, t.edit_card.bg, 0.48),
                    hover: lerp_color(base_c.hover, t.edit_card.hover, 0.48),
                    border: t.edit_card.border,
                    accent: lerp_color(base_c.accent, t.edit_card.accent, 0.45),
                }
            } else {
                base_c
            };

            let fill = lerp_color(target_c.bg, target_c.hover, hover_t);
            let border_col = lerp_color(target_c.border, target_c.accent, hover_t * 0.55);

            let painter = ui.painter();
            painter.rect_filled(q_rect, card_r, fill);
            painter.rect_stroke(q_rect, card_r, Stroke::new(t.stroke_w, border_col));

            let content_rect = Rect::from_min_max(
                Pos2::new(q_rect.min.x + pad.left, q_rect.min.y + pad.top),
                Pos2::new(q_rect.max.x - pad.right, q_rect.max.y - pad.bottom),
            );
            ui.allocate_ui_at_rect(content_rect, |ui| {
                self.render_quadrant_content(ui, panel_id, target_c.accent, t);
            });

            if resp.clicked() && !pointer_in_cycle {
                self.state.active_panel = Some(panel_id);
            }
        }

        // Dead-Center Circular Cycle Dial
        let alloc_pad = t.sp(4.0);
        egui::Area::new(Id::new("cycle_button"))
            .order(Order::Foreground)
            .fixed_pos(center - Vec2::splat(ring_r + alloc_pad))
            .show(ctx, |ui| {
                let (ring_rect, _) =
                    ui.allocate_exact_size(Vec2::splat((ring_r + alloc_pad) * 2.0), Sense::hover());
                let inner_rect =
                    Rect::from_center_size(ring_rect.center(), Vec2::splat(inner_r * 2.0));
                let resp = ui.interact(inner_rect, ui.id().with("cycle_inner"), Sense::click());
                if resp.hovered() {
                    ctx.set_cursor_icon(CursorIcon::PointingHand);
                }

                let c_hover = ctx.animate_bool_with_time(Id::new("cycle_hover"), resp.hovered(), t.anim_secs);
                let painter = ui.painter();
                let c = ring_rect.center();

                painter.circle(c, ring_r, t.screen_bg, Stroke::new(t.stroke_w, t.hairline));

                let (base_fill, hover_fill, ring_col, mode_lbl) = if is_edit {
                    (t.edit_card.bg, t.edit_card.hover, t.accent_input, "Edit mode")
                } else {
                    (t.raised_bg, t.raised_hover, t.accent_view, "View mode")
                };
                let fill_col = lerp_color(base_fill, hover_fill, c_hover);
                let icon_col = if is_edit { t.accent_input } else { t.text_main };

                painter.circle(c, inner_r, fill_col, Stroke::new(t.stroke_w + t.sp(1.0), ring_col));
                draw_refresh_icon(
                    painter,
                    Pos2::new(c.x, c.y - inner_r * 0.44),
                    inner_r * 0.14,
                    icon_col,
                    t,
                );

                let dial_scale = self.state.a11y.text_scale.factor().min(1.06);
                painter.text(
                    Pos2::new(c.x, c.y - inner_r * 0.08),
                    Align2::CENTER_CENTER,
                    "Cycle",
                    heading_font((inner_r * 0.22).clamp(11.0, 34.0) * dial_scale),
                    t.text_main,
                );
                painter.text(
                    Pos2::new(c.x, c.y + inner_r * 0.17),
                    Align2::CENTER_CENTER,
                    mode_lbl,
                    FontId::proportional((inner_r * 0.14).clamp(9.0, 22.0) * dial_scale),
                    ring_col,
                );
                painter.text(
                    Pos2::new(c.x, c.y + inner_r * 0.48),
                    Align2::CENTER_CENTER,
                    self.state.phase.name(),
                    FontId::monospace((inner_r * 0.14).clamp(8.5, 22.0) * dial_scale),
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

    fn render_quadrant_content(&self, ui: &mut Ui, panel: PanelId, accent: Color32, t: Tokens) {
        let is_input = self.state.mode == Mode::Input;
        let ration_short = self.state.ration_short();
        let rep = self.state.overall_suit_repairability();

        let title = match panel {
            PanelId::Health => "Health & suit",
            PanelId::Hub => "Hub & nav",
            PanelId::Tech => "Comms & display",
            PanelId::Hazard => "Hazard & env",
        };

        // Clean compact header without 01/02/03/04 badges
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(title)
                    .font(heading_font(t.sz(14.0)))
                    .color(t.text_main),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let (tag_txt, tag_col) = if is_input {
                    ("Edit", t.accent_input)
                } else {
                    ("Live", accent)
                };
                ui.label(RichText::new(tag_txt).size(t.sz(10.5)).color(tag_col));
            });
        });

        ui.add_space(t.sp(3.0));
        ui.painter().line_segment(
            [
                Pos2::new(ui.min_rect().left(), ui.cursor().top()),
                Pos2::new(ui.max_rect().right(), ui.cursor().top()),
            ],
            Stroke::new(1.0_f32, accent.gamma_multiply(0.30)),
        );

        // Full remaining space used directly (no bottom footer strip!)
        ui.allocate_ui_with_layout(
            ui.available_size(),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.vertical(|ui| match panel {
                    PanelId::Health => {
                        if !is_input {
                            let (lvl, suit_col, suit_txt) = if rep == Repairability::AirlockRequired
                                || self.state.hazard == Hazard::Hypoxia
                            {
                                (StatusLevel::Critical, t.critical, "Airlock req.")
                            } else if rep == Repairability::FieldRepairable {
                                (StatusLevel::Caution, t.caution, "Field repair")
                            } else if self.state.phase == Phase::ReEntry {
                                (StatusLevel::Caution, t.caution, "Fluid load")
                            } else {
                                (StatusLevel::Nominal, accent, "Suit intact")
                            };

                            ui.horizontal(|ui| {
                                draw_heart_icon(ui, t.sp(18.0), suit_col, t);
                                ui.label(
                                    RichText::new(format!("{}", self.state.live_hr))
                                        .family(FontFamily::Monospace)
                                        .size(t.sz(28.0))
                                        .color(t.text_main),
                                );
                                ui.label(
                                    RichText::new("bpm")
                                        .family(FontFamily::Monospace)
                                        .size(t.sz(11.0))
                                        .color(t.text_muted),
                                );
                            });
                            let (lo, hi) = self.state.phase.hr_bounds();
                            draw_corridor_bar(
                                ui,
                                (ui.available_width() * 0.84).max(t.sp(90.0)),
                                self.state.live_hr as f32,
                                lo as f32,
                                hi as f32,
                                suit_col,
                                t,
                            );
                            ui.add_space(t.sp(4.0));
                            ui.horizontal(|ui| {
                                status_chip(ui, lvl, suit_txt, suit_col, t);
                                ui.label(
                                    RichText::new(format!("· BP {}", self.state.phase.bp()))
                                        .family(FontFamily::Monospace)
                                        .size(t.sz(11.0))
                                        .color(t.text_muted),
                                );
                            });
                        } else {
                            status_chip(ui, StatusLevel::Caution, "Meal, fluids & suit", t.accent_input, t);
                            ui.add_space(t.sp(5.0));
                            ui.label(
                                RichText::new(format!(
                                    "{}% meal · {} mL · {}",
                                    self.state.meal_eaten_pct,
                                    self.state.water_ml,
                                    self.state.suit_fault.sub()
                                ))
                                .family(FontFamily::Monospace)
                                .size(t.sz(13.0))
                                .color(t.text_main),
                            );
                        }
                    }
                    PanelId::Hub => {
                        if !is_input {
                            // Full space split cleanly between Distance/Waypoint info and large working Compass!
                            let dial_sz = (ui.available_height() * 0.86).clamp(t.sp(52.0), t.sp(110.0));
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!("{}", self.state.distance_m))
                                                .family(FontFamily::Monospace)
                                                .size(t.sz(28.0))
                                                .color(t.text_main),
                                        );
                                        ui.label(
                                            RichText::new("m")
                                                .family(FontFamily::Monospace)
                                                .size(t.sz(12.0))
                                                .color(t.text_muted),
                                        );
                                    });
                                    ui.label(
                                        RichText::new(format!(
                                            "{} · {:03.0}° {}",
                                            self.state.waypoint.label(),
                                            self.state.waypoint.bearing_deg(),
                                            self.state.waypoint.cardinal()
                                        ))
                                        .family(FontFamily::Monospace)
                                        .size(t.sz(11.5))
                                        .color(t.text_body),
                                    );
                                    ui.label(
                                        RichText::new(format!("{} · {}", self.state.phase.name(), ration_short))
                                            .size(t.sz(11.5))
                                            .color(accent),
                                    );
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    draw_working_compass(
                                        ui,
                                        dial_sz,
                                        self.state.compass_heading_deg,
                                        self.state.waypoint.bearing_deg(),
                                        accent,
                                        false,
                                        t,
                                    );
                                });
                            });
                        } else {
                            status_chip(ui, StatusLevel::Caution, "Phase & waypoint", t.accent_input, t);
                            ui.add_space(t.sp(5.0));
                            ui.label(
                                RichText::new(format!("{} · {} {}", self.state.phase.name(), self.state.waypoint.label(), self.state.distance_m))
                                    .family(FontFamily::Monospace)
                                    .size(t.sz(13.5))
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
                                        status_chip(
                                            ui,
                                            StatusLevel::Caution,
                                            &format!("Offline · {} pkts", self.state.buffered_packets),
                                            t.caution,
                                            t,
                                        );
                                    }
                                    ui.add_space(t.sp(3.0));
                                    if self.state.hazard != Hazard::Nominal {
                                        status_chip(ui, StatusLevel::Critical, "Protocol alert", t.critical, t);
                                    } else {
                                        ui.label(
                                            RichText::new("NASA-STD-3001 OK")
                                                .family(FontFamily::Monospace)
                                                .size(t.sz(11.5))
                                                .color(accent),
                                        );
                                    }
                                    ui.add_space(t.sp(2.0));
                                    ui.label(
                                        RichText::new(self.state.a11y.summary())
                                            .family(FontFamily::Monospace)
                                            .size(t.sz(10.5))
                                            .color(t.text_muted),
                                    );
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    draw_signal_bars(ui, t.sp(36.0), self.state.online, accent, t.caution, t);
                                });
                            });
                        } else {
                            status_chip(ui, StatusLevel::Caution, "Display & radio", t.accent_input, t);
                            ui.add_space(t.sp(5.0));
                            ui.label(
                                RichText::new(self.state.a11y.summary())
                                    .family(FontFamily::Monospace)
                                    .size(t.sz(13.0))
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
                            ui.add_space(t.sp(7.0));
                            ui.horizontal(|ui| {
                                gauge_pill(
                                    ui,
                                    "Rad",
                                    self.state.hazard.rad_val(),
                                    self.state.hazard.rad_ratio(),
                                    self.state.hazard == Hazard::Radiation,
                                    t,
                                );
                                ui.add_space(t.sp(10.0));
                                gauge_pill(
                                    ui,
                                    "CO2",
                                    self.state.hazard.co2_val(),
                                    self.state.hazard.co2_ratio(),
                                    self.state.hazard == Hazard::HighCo2,
                                    t,
                                );
                                ui.add_space(t.sp(10.0));
                                gauge_pill(
                                    ui,
                                    "O2",
                                    self.state.hazard.o2(),
                                    self.state.hazard.o2_ratio(),
                                    self.state.hazard == Hazard::Hypoxia,
                                    t,
                                );
                            });
                        } else {
                            status_chip(ui, StatusLevel::Caution, "Hazard tests", t.accent_input, t);
                            ui.add_space(t.sp(5.0));
                            ui.label(
                                RichText::new(self.state.hazard.label())
                                    .size(t.sz(14.0))
                                    .color(t.text_main),
                            );
                        }
                    }
                });
            },
        );
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
        let inner = screen_rect.shrink(t.sp(12.0));
        let card_c = t.card_colors(panel);

        if !is_input {
            // View screens are read-only: whole body is content, a tap anywhere returns to the grid.
            ui.allocate_ui_at_rect(inner, |ui| match panel {
                PanelId::Health => self.overlay_health(ui, ctx, false, card_c, t),
                PanelId::Hub => self.overlay_hub(ui, ctx, false, card_c, t),
                PanelId::Tech => self.overlay_tech(ui, ctx, false, card_c, t),
                PanelId::Hazard => self.overlay_hazard(ui, ctx, false, card_c, t),
            });
            let resp = ui.interact(screen_rect, Id::new("overlay_tap_return"), Sense::click());
            if resp.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if resp.clicked() {
                self.state.active_panel = None;
            }
            return;
        }

        // Edit screens: no title, controls fill the body, one extra-large full-width Return bar at the bottom.
        let gap = t.sp(8.0);
        let ret_h = (inner.height() * 0.19).clamp(t.button_h * 1.45, t.sp(120.0));
        let ret_rect = Rect::from_min_max(Pos2::new(inner.left(), inner.bottom() - ret_h), inner.max);
        let body_rect = Rect::from_min_max(inner.min, Pos2::new(inner.right(), ret_rect.top() - gap));

        ui.allocate_ui_at_rect(body_rect, |ui| match panel {
            PanelId::Health => self.overlay_health(ui, ctx, true, card_c, t),
            PanelId::Hub => self.overlay_hub(ui, ctx, true, card_c, t),
            PanelId::Tech => self.overlay_tech(ui, ctx, true, card_c, t),
            PanelId::Hazard => self.overlay_hazard(ui, ctx, true, card_c, t),
        });

        let resp = ui.interact(ret_rect, Id::new("overlay_return_btn"), Sense::click());
        if resp.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        let h_t = ctx.animate_bool_with_time(Id::new("ret_hover"), resp.hovered(), t.anim_secs);
        let fill = lerp_color(card_c.bg, card_c.hover, h_t);
        let p = ui.painter();
        p.rect(ret_rect, t.sp(9.0), fill, Stroke::new(t.stroke_w * 1.6, card_c.accent));
        let pref = (t.text_mult / t.ui_scale.max(0.01)).clamp(0.8, 1.5);
        p.text(
            ret_rect.center(),
            Align2::CENTER_CENTER,
            "←  Return",
            FontId::proportional((ret_h * 0.42 * pref).clamp(16.0, 48.0)),
            t.text_main,
        );
        if resp.clicked() {
            self.state.active_panel = None;
        }
    }

    fn overlay_health(
        &mut self,
        ui: &mut Ui,
        ctx: &egui::Context,
        is_input: bool,
        card_c: CardColors,
        t: Tokens,
    ) {
        let p = self.state.phase;
        let zones = self.state.suit_zones();
        let overall_rep = self.state.overall_suit_repairability();
        let body_h = ui.available_height();
        let gap = t.sp(8.0);

        if !is_input {
            let top_h = (body_h * 0.27).clamp(t.sp(58.0), t.sp(118.0));
            ui.columns(4, |c| {
                stat_card_h(&mut c[0], top_h, "Heart rate", &format!("{} bpm", self.state.live_hr), card_c.accent, p.hr_range(), card_c, t);
                stat_card_h(&mut c[1], top_h, "Blood pressure", p.bp(), t.text_main, "Optical", card_c, t);
                stat_card_h(&mut c[2], top_h, "Calorie deficit", &format!("{} kcal", self.state.calorie_deficit), t.caution, &format!("{:.1}/min", p.kcal_rate()), card_c, t);
                stat_card_h(&mut c[3], top_h, "Hydration", &format!("-{} mL", self.state.sweat_deficit_ml), t.accent_view, &format!("+{} mL in", self.state.water_ml), card_c, t);
            });

            ui.add_space(gap);

            // Remaining 100% height for Spacesuit Schematic + Suit Damage & Repair (zero rag_box filler!)
            let mid_h = (body_h - top_h - gap - t.sp(2.0)).max(t.sp(120.0));
            ui.columns(2, |cols| {
                let left_rect = cols[0].allocate_exact_size(Vec2::new(cols[0].available_width(), mid_h), Sense::hover()).0;
                cols[0].painter().rect(left_rect, t.sp(8.0), card_c.bg, Stroke::new(t.stroke_w, card_c.border));
                let left_in = left_rect.shrink(t.sp(10.0));
                cols[0].allocate_ui_at_rect(left_in, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Spacesuit schematic").font(heading_font(t.sz(13.0))).color(t.text_main));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let (lvl, col, msg) = match overall_rep {
                                Repairability::Intact => (StatusLevel::Nominal, t.nominal, "100% Intact"),
                                Repairability::FieldRepairable => (StatusLevel::Caution, t.caution, "Field repairable"),
                                Repairability::AirlockRequired => (StatusLevel::Critical, t.critical, "Airlock req."),
                            };
                            status_chip(ui, lvl, msg, col, t);
                        });
                    });
                    let schem_h = (ui.available_height() - t.sp(4.0)).max(t.sp(70.0));
                    ui.centered_and_justified(|ui| {
                        draw_suit_schematic(ui, schem_h, &zones, t);
                    });
                });

                let right_rect = cols[1].allocate_exact_size(Vec2::new(cols[1].available_width(), mid_h), Sense::hover()).0;
                cols[1].painter().rect(right_rect, t.sp(8.0), card_c.bg, Stroke::new(t.stroke_w, card_c.border));
                let right_in = right_rect.shrink(t.sp(10.0));
                cols[1].allocate_ui_at_rect(right_in, |ui| {
                    ui.label(RichText::new("Suit damage & repair").font(heading_font(t.sz(13.0))).color(t.text_main));
                    ui.add_space(t.sp(4.0));
                    let row_avail = (ui.available_height() - t.sp(6.0)).max(t.sp(75.0));
                    let pref = (t.text_mult / t.ui_scale.max(0.01)).clamp(0.8, 1.5);
                    let zone_sz = (row_avail / 5.0 * 0.30 * pref).clamp(11.0, 26.0);
                    let act_sz = (zone_sz * 0.82).max(10.0);
                    let row_step = (row_avail / 5.0 - zone_sz * 2.2).clamp(t.sp(2.0), t.sp(18.0));
                    for z in &zones {
                        let (lvl, col, badge) = match z.repairability {
                            Repairability::Intact => (StatusLevel::Nominal, t.nominal, "Intact"),
                            Repairability::FieldRepairable => (StatusLevel::Caution, t.caution, "Field repair"),
                            Repairability::AirlockRequired => (StatusLevel::Critical, t.critical, "Unrepairable"),
                        };
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(Vec2::splat(t.sp(8.0).max(6.0)), Sense::hover());
                            draw_status_shape(ui.painter(), r.center(), lvl, col, t);
                            ui.label(RichText::new(z.zone).size(zone_sz).color(t.text_main));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(
                                    RichText::new(format!("{}% · {}", z.integrity_pct, badge))
                                        .family(FontFamily::Monospace)
                                        .size(act_sz)
                                        .color(col),
                                );
                            });
                        });
                        if z.repairability != Repairability::Intact {
                            ui.label(RichText::new(z.action).size(act_sz).color(t.text_muted));
                        }
                        ui.add_space(row_step * 0.45);
                    }
                });
            });
        } else {
            // Non-scrolling heading-free 2-column Edit layout where buttons fill 100% height cleanly
            let left_rows = if overall_rep == Repairability::FieldRepairable { 5.0 } else { 4.0 };
            let left_btn_h = ((body_h - gap * (left_rows - 1.0)) / left_rows).max(t.sp(30.0));
            let right_btn_h = ((body_h - gap * 3.0) / 4.0).max(t.sp(30.0));

            ui.columns(2, |cols| {
                cols[0].vertical(|ui| {
                    for (i, &fault) in SuitFault::ALL.iter().enumerate() {
                        if color_button_h(ui, ctx, ("fault_btn", i), left_btn_h, fault.label(), fault.sub(), self.state.suit_fault == fault, card_c, t).clicked() {
                            self.state.suit_fault = fault;
                        }
                        if i < 3 || overall_rep == Repairability::FieldRepairable {
                            ui.add_space(gap);
                        }
                    }
                    if overall_rep == Repairability::FieldRepairable {
                        if color_button_h(ui, ctx, "edit_repair_btn", left_btn_h, "Apply EVA field repair", "Restore 100%", true, card_c, t).clicked() {
                            self.state.apply_field_repair();
                        }
                    }
                });

                cols[1].vertical(|ui| {
                    for pair in [100_u32, 50, 25, 0].chunks(2) {
                        ui.columns(2, |c| {
                            for (i, &pct) in pair.iter().enumerate() {
                                let sub = match pct {
                                    100 => "Full",
                                    0 => "Skipped",
                                    _ => "Partial",
                                };
                                if color_button_h(&mut c[i], ctx, ("meal_btn", pct), right_btn_h, &format!("Meal {pct}%"), sub, self.state.meal_eaten_pct == pct, card_c, t).clicked() {
                                    self.state.set_meal(pct);
                                }
                            }
                        });
                        ui.add_space(gap);
                    }

                    if color_button_h(ui, ctx, "water_btn", right_btn_h, "+ 250 mL water", &format!("{} / 1,150 mL", self.state.water_ml), false, card_c, t).clicked() {
                        self.state.water_ml += 250;
                        self.state.sweat_deficit_ml = self.state.sweat_deficit_ml.saturating_sub(250);
                    }
                    ui.add_space(gap);
                    if color_button_h(ui, ctx, "nacl_btn", right_btn_h, "+ 1g salt tablet", &format!("{} / 9 tablets", self.state.nacl_count), false, card_c, t).clicked() {
                        if self.state.nacl_count < 9 {
                            self.state.nacl_count += 1;
                        }
                    }
                });
            });
        }
    }

    fn overlay_hub(
        &mut self,
        ui: &mut Ui,
        ctx: &egui::Context,
        is_input: bool,
        card_c: CardColors,
        t: Tokens,
    ) {
        let p = self.state.phase;
        let body_h = ui.available_height();
        let gap = t.sp(8.0);

        if !is_input {
            // Left: Full-height live compass panel (read-only so tapping anywhere returns!)
            // Right: 4 metric tiles (2x2) growing cleanly to fill 100% of remaining height
            ui.columns(2, |cols| {
                let left_rect = cols[0].allocate_exact_size(Vec2::new(cols[0].available_width(), body_h), Sense::hover()).0;
                cols[0].painter().rect(left_rect, t.sp(8.0), card_c.bg, Stroke::new(t.stroke_w, card_c.border));
                let left_in = left_rect.shrink(t.sp(10.0));
                cols[0].allocate_ui_at_rect(left_in, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Surface compass").font(heading_font(t.sz(13.0))).color(t.text_main));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!("Hdg {:03.0}° · Rel {:03.0}°", self.state.compass_heading_deg, self.state.relative_bearing_deg()))
                                    .family(FontFamily::Monospace)
                                    .size(t.sz(11.0))
                                    .color(card_c.accent),
                            );
                        });
                    });

                    let hdr_used = t.sz(13.0) + t.sp(10.0);
                    let middle_h = (left_in.height() - hdr_used).max(t.sp(70.0));
                    let dial_sz = (middle_h * 0.90)
                        .min(left_in.width() * 0.88)
                        .max(t.sp(64.0));

                    ui.allocate_ui(Vec2::new(ui.available_width(), middle_h), |ui| {
                        ui.centered_and_justified(|ui| {
                            draw_working_compass(
                                ui,
                                dial_sz,
                                self.state.compass_heading_deg,
                                self.state.waypoint.bearing_deg(),
                                card_c.accent,
                                true,
                                t,
                            );
                        });
                    });
                });

                cols[1].vertical(|ui| {
                    let tile_h = ((body_h - gap) * 0.5).max(t.sp(56.0));
                    ui.columns(2, |r1| {
                        stat_card_h(&mut r1[0], tile_h, "Target distance", &format!("{} m", self.state.distance_m), card_c.accent, self.state.waypoint.label(), card_c, t);
                        stat_card_h(&mut r1[1], tile_h, "Flight phase", p.name(), t.text_main, p.hr_range(), card_c, t);
                    });
                    ui.add_space(gap);
                    ui.columns(2, |r2| {
                        stat_card_h(&mut r2[0], tile_h, "Earth signal lag", "18m 42s", t.caution, "Local autonomous", card_c, t);
                        stat_card_h(
                            &mut r2[1],
                            tile_h,
                            "Surface LAN",
                            if self.state.online { "Connected" } else { "Offline" },
                            if self.state.online { t.nominal } else { t.caution },
                            "192.168.1.10",
                            card_c,
                            t,
                        );
                    });
                });
            });
        } else {
            let avail = (body_h - gap * 3.0).max(t.sp(120.0));
            let phase_h = avail * 0.28;
            let wp_h = avail * 0.22;
            let sync_h = avail * 0.22;

            for row in 0..2 {
                ui.columns(2, |c| {
                    for col in 0..2 {
                        let i = row * 2 + col;
                        let ph = Phase::ALL[i];
                        if color_button_h(&mut c[col], ctx, ("phase_btn", i), phase_h, ph.name(), ph.hr_range(), self.state.phase == ph, card_c, t).clicked() {
                            self.state.set_phase(ph);
                        }
                    }
                });
                ui.add_space(gap);
            }

            ui.columns(2, |c| {
                if color_button_h(&mut c[0], ctx, "wp_hab_e", wp_h, "Target: Habitat", "222 m · 042°", self.state.waypoint == NavWaypoint::Habitat, card_c, t).clicked() {
                    self.state.set_waypoint(NavWaypoint::Habitat);
                }
                if color_button_h(&mut c[1], ctx, "wp_air_e", wp_h, "Target: Airlock", "145 m · 312°", self.state.waypoint == NavWaypoint::Airlock, card_c, t).clicked() {
                    self.state.set_waypoint(NavWaypoint::Airlock);
                }
            });
            ui.add_space(gap);
            if color_button_h(ui, ctx, "sync_btn", sync_h, "Force sync telemetry to hub", &format!("Flush {} queued packets", self.state.buffered_packets), false, card_c, t).clicked() {
                self.state.buffered_packets = 0;
                self.state.online = true;
            }
        }
    }

    fn overlay_tech(
        &mut self,
        ui: &mut Ui,
        ctx: &egui::Context,
        is_input: bool,
        card_c: CardColors,
        t: Tokens,
    ) {
        let body_h = ui.available_height();
        let gap = t.sp(8.0);

        if !is_input {
            let tile_h = ((body_h - gap) * 0.5).max(t.sp(56.0));
            ui.columns(2, |r1| {
                stat_card_h(&mut r1[0], tile_h, "Safety engine", "1 Hz local", t.nominal, "Zero Earth lag", card_c, t);
                stat_card_h(
                    &mut r1[1],
                    tile_h,
                    "Surface LAN",
                    if self.state.online { "Online" } else { "Offline" },
                    if self.state.online { t.nominal } else { t.caution },
                    "Auto-sync",
                    card_c,
                    t,
                );
            });
            ui.add_space(gap);
            ui.columns(2, |r2| {
                stat_card_h(&mut r2[0], tile_h, "Offline queue", &format!("{} pkts", self.state.buffered_packets), t.text_main, "Local buffer", card_c, t);
                stat_card_h(&mut r2[1], tile_h, "Display profile", &self.state.a11y.summary(), card_c.accent, "Edit via Cycle", card_c, t);
            });
        } else {
            let left_row_h = ((body_h - 3.0 * gap) / 5.0).max(t.sp(24.0));
            let slider_h = left_row_h * 2.0;
            let right_row_h = ((body_h - 4.0 * gap) / 5.0).max(t.sp(24.0));

            ui.columns(2, |cols| {
                cols[0].vertical(|ui| {
                    if let Some(v) = brightness_slider(ui, slider_h, self.state.a11y.brightness_pct, card_c, t) {
                        self.state.a11y.brightness_pct = v;
                    }
                    ui.add_space(gap);
                    for (idx, &th) in ColorTheme::ALL.iter().enumerate() {
                        if color_button_h(ui, ctx, ("theme_btn", th as u8), left_row_h, th.label(), if self.state.a11y.theme == th { "Active" } else { "" }, self.state.a11y.theme == th, card_c, t).clicked() {
                            self.state.a11y.theme = th;
                        }
                        if idx < ColorTheme::ALL.len() - 1 {
                            ui.add_space(gap);
                        }
                    }
                });

                cols[1].vertical(|ui| {
                    for pair in ColorVision::ALL.chunks(2) {
                        ui.columns(2, |c| {
                            for (i, &vis) in pair.iter().enumerate() {
                                if color_button_h(&mut c[i], ctx, ("vis_btn", vis as u8), right_row_h, vis.label(), vis.short(), self.state.a11y.vision == vis, card_c, t).clicked() {
                                    self.state.a11y.vision = vis;
                                }
                            }
                        });
                        ui.add_space(gap);
                    }

                    ui.columns(3, |c| {
                        for (i, &ts) in TextScalePreset::ALL.iter().enumerate() {
                            if color_button_h(&mut c[i], ctx, ("txt_btn", i), right_row_h, &format!("Text: {}", ts.label()), "", self.state.a11y.text_scale == ts, card_c, t).clicked() {
                                self.state.a11y.text_scale = ts;
                            }
                        }
                    });
                    ui.add_space(gap);
                    ui.columns(2, |c| {
                        if color_button_h(&mut c[0], ctx, "a11y_gl", right_row_h, "Glove+ size", if self.state.a11y.glove_targets { "ON" } else { "OFF" }, self.state.a11y.glove_targets, card_c, t).clicked() {
                            self.state.a11y.glove_targets = !self.state.a11y.glove_targets;
                        }
                        if color_button_h(&mut c[1], ctx, "a11y_rm", right_row_h, "Reduced motion", if self.state.a11y.reduced_motion { "ON" } else { "OFF" }, self.state.a11y.reduced_motion, card_c, t).clicked() {
                            self.state.a11y.reduced_motion = !self.state.a11y.reduced_motion;
                        }
                    });
                    ui.add_space(gap);
                    let lan_lbl = if self.state.online { "Test LAN blackout" } else { "Reconnect & flush" };
                    let lan_sub = if self.state.online { "Online" } else { "Offline" };
                    if color_button_h(ui, ctx, "lan_btn", right_row_h, lan_lbl, lan_sub, !self.state.online, card_c, t).clicked() {
                        self.state.online = !self.state.online;
                        if self.state.online {
                            self.state.buffered_packets = 0;
                        }
                    }
                });
            });
        }
    }

    fn overlay_hazard(
        &mut self,
        ui: &mut Ui,
        ctx: &egui::Context,
        is_input: bool,
        card_c: CardColors,
        t: Tokens,
    ) {
        let h = self.state.hazard;
        let body_h = ui.available_height();
        let gap = t.sp(8.0);

        if !is_input {
            let tile_h = ((body_h - gap) * 0.5).max(t.sp(56.0));
            ui.columns(2, |r1| {
                stat_card_h(&mut r1[0], tile_h, "Radiation", h.rad(), if h == Hazard::Radiation { t.critical } else { t.nominal }, "Limit < 2.0", card_c, t);
                stat_card_h(&mut r1[1], tile_h, "Suit CO2", h.co2(), if h == Hazard::HighCo2 { t.caution } else { t.nominal }, "Limit < 4.0", card_c, t);
            });
            ui.add_space(gap);
            ui.columns(2, |r2| {
                stat_card_h(&mut r2[0], tile_h, "Tissue oxygen", h.o2(), if h == Hazard::Hypoxia { t.critical } else { t.nominal }, "Safe >= 92%", card_c, t);
                stat_card_h(&mut r2[1], tile_h, "Surface wind", "3 m/s NE", t.text_main, "Visibility clear", card_c, t);
            });
        } else {
            let btn_h = ((body_h - gap) * 0.5).max(t.sp(48.0));
            for (row, pair) in Hazard::ALL.chunks(2).enumerate() {
                ui.columns(2, |c| {
                    for (i, hk) in pair.iter().enumerate() {
                        let sub = if *hk == Hazard::Nominal { "Clear" } else { "Trigger" };
                        if color_button_h(&mut c[i], ctx, ("haz_btn", row * 2 + i), btn_h, hk.label(), sub, self.state.hazard == *hk, card_c, t).clicked() {
                            self.state.hazard = *hk;
                        }
                    }
                });
                if row == 0 {
                    ui.add_space(gap);
                }
            }
        }
    }
}

// ───────────────────────── Visual & Instrument Primitives ─────────────────────────
fn fit_16_by_9(bounds: Rect) -> Rect {
    let target = 16.0_f32 / 9.0_f32;
    let (w, h) = if bounds.width() / bounds.height() > target {
        (bounds.height() * target, bounds.height())
    } else {
        (bounds.width(), bounds.width() / target)
    };
    Rect::from_center_size(bounds.center(), Vec2::new(w, h))
}

fn scale_brightness(c: Color32, factor: f32) -> Color32 {
    let f = factor.clamp(0.25, 1.75);
    let ch = |v: u8| -> u8 { (v as f32 * f).clamp(0.0, 255.0).round() as u8 };
    Color32::from_rgba_unmultiplied(ch(c.r()), ch(c.g()), ch(c.b()), c.a())
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| -> u8 { (x as f32 + (y as f32 - x as f32) * t).round() as u8 };
    Color32::from_rgba_unmultiplied(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()), mix(a.a(), b.a()))
}

fn zone_color(rep: Repairability, t: Tokens) -> Color32 {
    match rep {
        Repairability::Intact => t.nominal,
        Repairability::FieldRepairable => t.caution,
        Repairability::AirlockRequired => t.critical,
    }
}

fn draw_suit_schematic(
    ui: &mut Ui,
    height: f32,
    zones: &[SuitZoneStatus; 5],
    t: Tokens,
) {
    let s = height / 130.0;
    let width = 110.0 * s;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let p = ui.painter();
    let cx = rect.center().x;
    let top = rect.top() + 8.0 * s;

    let helmet_col = zone_color(zones[0].repairability, t);
    let torso_col = zone_color(zones[1].repairability, t);
    let scrub_col = zone_color(zones[2].repairability, t);
    let glove_col = zone_color(zones[3].repairability, t);
    let shield_col = zone_color(zones[4].repairability, t);

    let rad_r = 56.0 * s;
    let rad_w = if zones[4].repairability == Repairability::AirlockRequired {
        2.2_f32 * s
    } else {
        1.1_f32 * s
    };
    p.circle_stroke(rect.center(), rad_r, Stroke::new(rad_w, shield_col.gamma_multiply(0.55)));

    let pack = Rect::from_min_size(Pos2::new(cx - 28.0 * s, top + 34.0 * s), Vec2::new(10.0 * s, 38.0 * s));
    p.rect(pack, 3.0 * s, scrub_col.gamma_multiply(0.24), Stroke::new(1.5_f32 * s, scrub_col));

    let head_c = Pos2::new(cx, top + 17.0 * s);
    p.circle(head_c, 15.0 * s, t.screen_bg, Stroke::new(1.8_f32 * s, helmet_col));
    p.rect_filled(
        Rect::from_center_size(Pos2::new(cx + 2.0 * s, head_c.y), Vec2::new(16.0 * s, 9.5 * s)),
        3.5 * s,
        helmet_col,
    );

    let torso = Rect::from_min_size(Pos2::new(cx - 16.0 * s, top + 35.0 * s), Vec2::new(32.0 * s, 40.0 * s));
    p.rect(torso, 6.0 * s, torso_col.gamma_multiply(0.20), Stroke::new(1.8_f32 * s, torso_col));
    p.circle_filled(torso.center(), 4.5 * s, torso_col);

    let l_shoulder = Pos2::new(torso.left(), torso.top() + 5.0 * s);
    let l_hand = Pos2::new(torso.left() - 12.0 * s, torso.top() + 30.0 * s);
    p.line_segment([l_shoulder, l_hand], Stroke::new(4.2_f32 * s, torso_col));
    p.circle_filled(l_hand, 4.5 * s, glove_col);

    let r_shoulder = Pos2::new(torso.right(), torso.top() + 5.0 * s);
    let r_hand = Pos2::new(torso.right() + 12.0 * s, torso.top() + 30.0 * s);
    p.line_segment([r_shoulder, r_hand], Stroke::new(4.2_f32 * s, torso_col));
    p.circle_filled(r_hand, 4.5 * s, glove_col);

    p.line_segment(
        [Pos2::new(cx - 8.0 * s, torso.bottom()), Pos2::new(cx - 10.0 * s, torso.bottom() + 34.0 * s)],
        Stroke::new(5.5_f32 * s, torso_col),
    );
    p.line_segment(
        [Pos2::new(cx + 8.0 * s, torso.bottom()), Pos2::new(cx + 10.0 * s, torso.bottom() + 34.0 * s)],
        Stroke::new(5.5_f32 * s, torso_col),
    );
}

fn draw_working_compass(
    ui: &mut Ui,
    size: f32,
    heading_deg: f32,
    target_deg: f32,
    color: Color32,
    detailed: bool,
    t: Tokens,
) {
    let s = size / 68.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let c = rect.center();
    let r = size * 0.44;
    let p = ui.painter();

    p.circle(c, r, t.screen_bg.gamma_multiply(0.65), Stroke::new(t.stroke_w, t.hairline_strong));

    // Degree tick ring
    let ticks = if detailed { 12 } else { 8 };
    for k in 0..ticks {
        let ang = (k as f32) * (std::f32::consts::TAU / ticks as f32);
        let d = Vec2::new(ang.sin(), -ang.cos());
        let len = if k % (ticks / 4) == 0 { 5.0 * s } else { 2.5 * s };
        p.line_segment([c + d * (r - len), c + d * r], Stroke::new(1.1_f32 * s, t.text_muted));
    }

    // Cardinal labels (N/E/S/W rotated relative to live suit heading so compass visibly tracks turns!)
    let cards = [("N", 0.0_f32), ("E", 90.0), ("S", 180.0), ("W", 270.0)];
    let lbl_size = if detailed { 10.5 * s } else { 8.5 * s };
    for (lbl, deg) in cards {
        let rel_rad = (deg - heading_deg).to_radians();
        let pos = c + Vec2::new(rel_rad.sin(), -rel_rad.cos()) * (r - 10.0 * s);
        let col = if lbl == "N" { t.caution } else { t.text_muted };
        p.text(pos, Align2::CENTER_CENTER, lbl, FontId::monospace(lbl_size.max(8.0)), col);
    }

    // Live target waypoint needle
    let rel_target = (target_deg - heading_deg).to_radians();
    let dir = Vec2::new(rel_target.sin(), -rel_target.cos());
    let tip = c + dir * (r * 0.72);
    let tail = c - dir * (r * 0.25);
    p.line_segment([tail, tip], Stroke::new(2.0_f32 * s, color));
    let perp = Vec2::new(-dir.y, dir.x);
    let head1 = tip - dir * (6.0 * s) + perp * (4.0 * s);
    let head2 = tip - dir * (6.0 * s) - perp * (4.0 * s);
    p.add(Shape::convex_polygon(vec![tip, head1, head2], color, Stroke::NONE));
    p.circle_filled(c, 2.5 * s, t.text_main);
}

fn draw_status_shape(painter: &egui::Painter, center: Pos2, level: StatusLevel, color: Color32, t: Tokens) {
    let r = t.sp(4.0).max(3.2);
    match level {
        StatusLevel::Nominal => {
            painter.circle_filled(center, r, color);
        }
        StatusLevel::Caution => {
            let pts = vec![
                Pos2::new(center.x, center.y - r * 1.1),
                Pos2::new(center.x + r * 1.1, center.y + r * 0.9),
                Pos2::new(center.x - r * 1.1, center.y + r * 0.9),
            ];
            painter.add(Shape::convex_polygon(pts, color, Stroke::NONE));
        }
        StatusLevel::Critical => {
            let rect = Rect::from_center_size(center, Vec2::splat(r * 2.0));
            painter.rect_filled(rect, 1.5, color);
        }
    }
}

fn status_chip(ui: &mut Ui, level: StatusLevel, label: &str, color: Color32, t: Tokens) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::splat(t.sp(9.5).max(7.5)), Sense::hover());
        draw_status_shape(ui.painter(), r.center(), level, color, t);
        ui.label(RichText::new(label).font(heading_font(t.sz(12.5))).color(color));
    });
}

fn draw_signal_bars(ui: &mut Ui, size: f32, online: bool, on_col: Color32, off_col: Color32, t: Tokens) {
    let s = size / 30.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let p = ui.painter();
    let active_bars = if online { 4 } else { 1 };
    let col = if online { on_col } else { off_col };
    let w = 4.5 * s;
    let gap = 2.5 * s;
    let start_x = rect.left() + 2.0 * s;
    for i in 0..4 {
        let h = (6.0 + (i as f32) * 5.0) * s;
        let x = start_x + (i as f32) * (w + gap);
        let bar = Rect::from_min_size(Pos2::new(x, rect.bottom() - 3.0 * s - h), Vec2::new(w, h));
        let fill = if i < active_bars { col } else { t.hairline };
        p.rect_filled(bar, 1.0 * s, fill);
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
    let bar_h = t.sp(4.5).max(3.5);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, t.sp(9.0).max(7.0)), Sense::hover());
    let p = ui.painter();
    let track = Rect::from_min_size(Pos2::new(rect.left(), rect.center().y - bar_h * 0.5), Vec2::new(width, bar_h));
    p.rect_filled(track, bar_h * 0.5, t.hairline);

    let safe_left = track.left() + width * 0.20;
    let safe_w = width * 0.60;
    let safe_rect = Rect::from_min_size(Pos2::new(safe_left, track.top()), Vec2::new(safe_w, bar_h));
    p.rect_filled(safe_rect, bar_h * 0.5, t.hairline_strong);

    let span = (hi - lo).max(1.0);
    let norm = ((val - lo) / span).clamp(0.0, 1.0);
    let x = safe_left + safe_w * norm;
    p.circle_filled(Pos2::new(x, track.center().y), t.sp(3.8).max(3.0), marker_col);
}

fn gauge_pill(
    ui: &mut Ui,
    name: &str,
    val: &str,
    ratio: f32,
    alerted: bool,
    t: Tokens,
) {
    let col = if alerted { t.critical } else { t.text_main };
    let bar_col = if alerted { t.critical } else { t.nominal };
    ui.vertical(|ui| {
        ui.label(
            RichText::new(format!("{name} {val}"))
                .family(FontFamily::Monospace)
                .size(t.sz(11.5))
                .color(col),
        );
        let (rect, _) = ui.allocate_exact_size(Vec2::new(t.sp(60.0).max(42.0), t.sp(4.5).max(3.5)), Sense::hover());
        ui.painter().rect_filled(rect, 2.0, t.hairline);
        let fill = Rect::from_min_size(rect.min, Vec2::new(rect.width() * ratio.clamp(0.05, 1.0), rect.height()));
        ui.painter().rect_filled(fill, 2.0, bar_col);
    });
}

fn stat_card_h(
    ui: &mut Ui,
    height: f32,
    label: &str,
    value: &str,
    val_color: Color32,
    note: &str,
    card_c: CardColors,
    t: Tokens,
) {
    let rect = ui
        .allocate_exact_size(
            Vec2::new(ui.available_width(), height.max(36.0)),
            Sense::hover(),
        )
        .0;
    let p = ui.painter();
    p.rect(rect, t.sp(8.0), card_c.bg, Stroke::new(t.stroke_w, card_c.border));

    // Text sizes driven by tile geometry so the value fills the tile; user text preset still applies.
    let pref = (t.text_mult / t.ui_scale.max(0.01)).clamp(0.8, 1.5);
    let pad = Vec2::new((rect.width() * 0.06).clamp(8.0, 24.0), (rect.height() * 0.09).clamp(6.0, 20.0));
    let inner = rect.shrink2(pad);
    let has_note = !note.is_empty();

    let lbl_sz = (inner.height() * 0.13 * pref).clamp(11.0, 30.0);
    let note_sz = (inner.height() * 0.11 * pref).clamp(10.0, 24.0);
    let chars = value.chars().count().max(1) as f32;
    let fit_w = inner.width() / (chars * 0.62);
    let val_room = inner.height() - lbl_sz - if has_note { note_sz } else { 0.0 } - 6.0;
    let val_sz = (val_room * 0.92 * pref).min(fit_w).min(val_room).clamp(12.0, 140.0);

    p.text(inner.left_top(), Align2::LEFT_TOP, label, FontId::proportional(lbl_sz), t.text_muted);
    let val_center_y = if has_note {
        inner.top() + lbl_sz + (inner.height() - lbl_sz - note_sz) * 0.5
    } else {
        inner.top() + lbl_sz + (inner.height() - lbl_sz) * 0.5
    };
    p.text(
        Pos2::new(inner.left(), val_center_y),
        Align2::LEFT_CENTER,
        value,
        FontId::monospace(val_sz),
        val_color,
    );
    if has_note {
        p.text(inner.left_bottom(), Align2::LEFT_BOTTOM, note, FontId::monospace(note_sz), t.text_muted);
    }
}

/// Touch/drag brightness bar (25–175%). Returns the new value when it changes.
fn brightness_slider(ui: &mut Ui, height: f32, value: u16, card_c: CardColors, t: Tokens) -> Option<u16> {
    let (lo, hi) = (AccessibilityConfig::BRIGHTNESS_MIN, AccessibilityConfig::BRIGHTNESS_MAX);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height.max(40.0)), Sense::click_and_drag());
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
    }
    let p = ui.painter();
    let r = t.sp(8.0);
    p.rect(rect, r, card_c.bg, Stroke::new(t.stroke_w, card_c.border));

    let pad = (rect.height() * 0.14).clamp(5.0, 16.0);
    let track = rect.shrink(pad);
    let frac = (value.clamp(lo, hi) - lo) as f32 / (hi - lo) as f32;

    // filled progress
    let fill_rect = Rect::from_min_max(track.min, Pos2::new(track.left() + track.width() * frac, track.bottom()));
    p.rect_filled(track, r * 0.7, t.raised_bg);
    p.rect_filled(fill_rect, r * 0.7, card_c.accent.gamma_multiply(0.55));

    // 100% reference tick + quarter ticks
    for pct in [50_u16, 100, 150] {
        let x = track.left() + track.width() * (pct - lo) as f32 / (hi - lo) as f32;
        let w = if pct == 100 { 2.0_f32 } else { 1.0_f32 };
        p.line_segment([Pos2::new(x, track.top()), Pos2::new(x, track.bottom())], Stroke::new(w, t.hairline_strong));
    }

    // handle
    let hx = track.left() + track.width() * frac;
    p.line_segment([Pos2::new(hx, rect.top() + 2.0), Pos2::new(hx, rect.bottom() - 2.0)], Stroke::new(3.0_f32, t.text_main));

    let txt_sz = (track.height() * 0.34).min(track.width() * 0.068).clamp(12.0, 38.0);
    p.text(
        Pos2::new(track.left() + pad, track.center().y),
        Align2::LEFT_CENTER,
        format!("Brightness  {value}%"),
        FontId::monospace(txt_sz),
        t.text_main,
    );
    let tag = match value {
        0..=59 => "Night",
        60..=94 => "Dim",
        95..=124 => "Cabin",
        _ => "Solar",
    };
    p.text(
        Pos2::new(track.right() - pad, track.center().y),
        Align2::RIGHT_CENTER,
        tag,
        FontId::proportional((txt_sz * 0.72).max(11.0)),
        t.text_body,
    );

    if let Some(pos) = resp.interact_pointer_pos() {
        if resp.dragged() || resp.clicked() {
            let f = ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0);
            let raw = lo as f32 + f * (hi - lo) as f32;
            let snapped = ((raw / 5.0).round() * 5.0) as u16;
            let v = snapped.clamp(lo, hi);
            if v != value {
                return Some(v);
            }
        }
    }
    None
}

fn color_button_h(
    ui: &mut Ui,
    ctx: &egui::Context,
    anim_key: impl std::hash::Hash,
    height: f32,
    left: &str,
    right: &str,
    selected: bool,
    card_c: CardColors,
    t: Tokens,
) -> egui::Response {
    let desired = Vec2::new(ui.available_width(), height.max(24.0));
    let (rect, resp) = ui.allocate_exact_size(desired, Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    let h_t = ctx.animate_bool_with_time(Id::new(anim_key), resp.hovered(), t.anim_secs);
    let (bg, border, text_col) = if selected {
        (t.edit_card.hover, t.accent_input, t.accent_input)
    } else {
        (
            lerp_color(card_c.bg, card_c.hover, h_t),
            lerp_color(card_c.border, card_c.accent, h_t * 0.7),
            t.text_main,
        )
    };

    let pref = (t.text_mult / t.ui_scale.max(0.01)).clamp(0.8, 1.5);
    let inset = (desired.y * 0.24).clamp(8.0, 22.0);
    let avail_w = (desired.x - inset * 2.0).max(40.0);
    let left_chars = left.chars().count().max(1) as f32;
    let right_chars = right.chars().count() as f32;
    let w_factor = if right.is_empty() {
        left_chars * 0.56
    } else {
        left_chars * 0.56 + right_chars * 0.46 + 1.5
    };
    let max_by_w = (avail_w / w_factor).max(10.0);
    let left_sz = (desired.y * 0.34 * pref).min(max_by_w).clamp(11.5, 42.0);
    let right_sz = (left_sz * 0.72).clamp(9.5, 30.0);
    let p = ui.painter();
    p.rect(rect, t.sp(7.0), bg, Stroke::new(t.stroke_w, border));
    p.text(
        Pos2::new(rect.left() + inset, rect.center().y),
        Align2::LEFT_CENTER,
        left,
        FontId::proportional(left_sz),
        text_col,
    );
    if !right.is_empty() {
        p.text(
            Pos2::new(rect.right() - inset, rect.center().y),
            Align2::RIGHT_CENTER,
            right,
            FontId::monospace(right_sz),
            if selected { t.accent_input } else { t.text_muted },
        );
    }
    resp
}

fn draw_heart_icon(ui: &mut Ui, size: f32, color: Color32, t: Tokens) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let c = rect.center();
    let mut pts = Vec::with_capacity(48);
    for i in 0..48 {
        let ang = i as f32 / 48.0 * std::f32::consts::TAU;
        let x = 16.0 * ang.sin().powi(3);
        let y = 13.0 * ang.cos() - 5.0 * (2.0 * ang).cos() - 2.0 * (3.0 * ang).cos() - (4.0 * ang).cos();
        pts.push(Pos2::new(c.x + x * size / 36.0, c.y - (y + 2.0) * size / 36.0));
    }
    ui.painter().add(Shape::closed_line(pts, Stroke::new(1.5_f32 * t.ui_scale.clamp(0.8, 1.5), color)));
}

fn draw_refresh_icon(painter: &egui::Painter, center: Pos2, r: f32, color: Color32, t: Tokens) {
    let steps = 20;
    let mut pts = Vec::with_capacity(steps);
    let start = -0.3_f32;
    let span = 4.6_f32;
    for i in 0..steps {
        let a = start + span * (i as f32 / (steps - 1) as f32);
        pts.push(Pos2::new(center.x + a.cos() * r, center.y + a.sin() * r));
    }
    let w = (1.5_f32 * t.ui_scale).clamp(1.2, 2.8);
    painter.add(Shape::line(pts.clone(), Stroke::new(w, color)));
    if let Some(&end) = pts.last() {
        let ah = t.sp(4.0).max(3.0);
        painter.line_segment([end, Pos2::new(end.x - ah, end.y - ah * 0.4)], Stroke::new(w, color));
        painter.line_segment([end, Pos2::new(end.x - ah * 0.25, end.y + ah)], Stroke::new(w, color));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_scale_proportionally_with_viewport_and_brightness() {
        let small = Rect::from_min_size(Pos2::ZERO, Vec2::new(640.0, 360.0));
        let large = Rect::from_min_size(Pos2::ZERO, Vec2::new(1920.0, 1080.0));

        let t_small = Tokens::resolve(AccessibilityConfig::default(), small);
        let t_large = Tokens::resolve(AccessibilityConfig::default(), large);

        assert!(t_large.ui_scale > t_small.ui_scale * 2.0);
        assert!(t_large.sz(14.0) > t_small.sz(14.0));
        assert!(t_large.button_h > t_small.button_h);

        let dim_cfg = AccessibilityConfig {
            brightness_pct: 65,
            ..Default::default()
        };
        let t_dim = Tokens::resolve(dim_cfg, large);
        assert!(t_dim.card_colors(PanelId::Health).bg.g() < t_large.card_colors(PanelId::Health).bg.g());
    }

    #[test]
    fn suit_repairability_reflects_fault_and_clears_on_field_repair() {
        let mut s = BiomonState::default();
        assert_eq!(s.overall_suit_repairability(), Repairability::Intact);

        s.suit_fault = SuitFault::GloveTear;
        assert_eq!(s.overall_suit_repairability(), Repairability::FieldRepairable);
        s.apply_field_repair();
        assert_eq!(s.overall_suit_repairability(), Repairability::Intact);

        s.suit_fault = SuitFault::CoreBreach;
        assert_eq!(s.overall_suit_repairability(), Repairability::AirlockRequired);
        s.apply_field_repair();
        assert_eq!(s.overall_suit_repairability(), Repairability::AirlockRequired);
    }

    #[test]
    fn waypoint_switch_updates_distance_and_bearing() {
        let mut s = BiomonState::default();
        assert_eq!(s.distance_m, 222);
        s.set_waypoint(NavWaypoint::Airlock);
        assert_eq!(s.distance_m, 145);
        assert_eq!(s.waypoint.cardinal(), "NW");
    }

    #[test]
    fn full_color_cards_have_distinct_background_hues() {
        let r = Rect::from_min_size(Pos2::ZERO, Vec2::new(960.0, 540.0));
        let t = Tokens::resolve(AccessibilityConfig::default(), r);
        let bgs = [
            t.card_colors(PanelId::Health).bg,
            t.card_colors(PanelId::Hub).bg,
            t.card_colors(PanelId::Tech).bg,
            t.card_colors(PanelId::Hazard).bg,
        ];
        for i in 0..bgs.len() {
            for j in (i + 1)..bgs.len() {
                assert_ne!(bgs[i], bgs[j]);
            }
        }
    }
}
