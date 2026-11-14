use eframe::egui::{Color32, Rect};

// ───────────────────────── Layer 1: Primitive Palettes ─────────────────────────
pub(crate) mod palette {
    use eframe::egui::Color32;

    pub const VOID_950: Color32 = Color32::from_rgb(11, 15, 23);
    pub const SCREEN_900: Color32 = Color32::from_rgb(16, 22, 34);
    pub const VISOR_950: Color32 = Color32::from_rgb(0, 0, 0);
    pub const VISOR_900: Color32 = Color32::from_rgb(4, 6, 9);

    pub const SLATE_850: Color32 = Color32::from_rgb(26, 34, 50);
    pub const SLATE_800: Color32 = Color32::from_rgb(34, 44, 64);
    pub const SLATE_700: Color32 = Color32::from_rgb(52, 66, 92);
    pub const SLATE_600: Color32 = Color32::from_rgb(76, 94, 124);

    // Full-Card Subsystem Surfaces
    pub const HEALTH_BG: Color32 = Color32::from_rgb(13, 36, 33);
    pub const HEALTH_HOVER: Color32 = Color32::from_rgb(18, 52, 47);
    pub const HEALTH_BORDER: Color32 = Color32::from_rgb(29, 98, 86);
    pub const HEALTH_ACCENT: Color32 = Color32::from_rgb(45, 212, 191);

    pub const HUB_BG: Color32 = Color32::from_rgb(13, 31, 51);
    pub const HUB_HOVER: Color32 = Color32::from_rgb(18, 45, 72);
    pub const HUB_BORDER: Color32 = Color32::from_rgb(31, 92, 132);
    pub const HUB_ACCENT: Color32 = Color32::from_rgb(56, 189, 248);

    pub const COMMS_BG: Color32 = Color32::from_rgb(25, 23, 49);
    pub const COMMS_HOVER: Color32 = Color32::from_rgb(35, 32, 70);
    pub const COMMS_BORDER: Color32 = Color32::from_rgb(76, 68, 152);
    pub const COMMS_ACCENT: Color32 = Color32::from_rgb(167, 139, 250);

    pub const HAZARD_BG: Color32 = Color32::from_rgb(38, 27, 13);
    pub const HAZARD_HOVER: Color32 = Color32::from_rgb(54, 38, 18);
    pub const HAZARD_BORDER: Color32 = Color32::from_rgb(126, 82, 26);
    pub const HAZARD_ACCENT: Color32 = Color32::from_rgb(251, 146, 60);

    // Edit Mode Warm Tinted Surfaces
    pub const EDIT_BG: Color32 = Color32::from_rgb(36, 27, 13);
    pub const EDIT_HOVER: Color32 = Color32::from_rgb(50, 38, 18);
    pub const EDIT_BORDER: Color32 = Color32::from_rgb(196, 132, 24);

    // Critical Alert Surfaces
    pub const ALERT_BG: Color32 = Color32::from_rgb(48, 15, 20);
    pub const ALERT_HOVER: Color32 = Color32::from_rgb(66, 20, 27);

    // Text Ramp
    pub const TEXT_WHITE: Color32 = Color32::from_rgb(248, 250, 252);
    pub const TEXT_BODY: Color32 = Color32::from_rgb(209, 217, 230);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(148, 163, 184);

    // Semantics
    pub const NOMINAL_GREEN: Color32 = Color32::from_rgb(34, 197, 94);
    pub const CAUTION_AMBER: Color32 = Color32::from_rgb(245, 158, 11);
    pub const CRITICAL_RED: Color32 = Color32::from_rgb(239, 68, 68);

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

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PanelId {
    Health,
    Hub,
    Tech,
    Hazard,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum StatusLevel {
    Nominal,
    Caution,
    Critical,
}

// ───────────────────────── Layer 2 & 3: Accessibility + Responsive Tokens ─────────────────────────
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColorTheme {
    FullColorCards,
    NightDark,
    HighVisorAaa,
}

impl ColorTheme {
    pub(crate) const ALL: [ColorTheme; 3] =
        [Self::FullColorCards, Self::NightDark, Self::HighVisorAaa];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::FullColorCards => "Color Cards",
            Self::NightDark => "Night Dark",
            Self::HighVisorAaa => "Sun Visor",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColorVision {
    Standard,
    RedGreenSafe,
    BlueYellowSafe,
    Monochrome,
}

impl ColorVision {
    pub(crate) const ALL: [ColorVision; 4] = [
        Self::Standard,
        Self::RedGreenSafe,
        Self::BlueYellowSafe,
        Self::Monochrome,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Standard => "Standard",
            Self::RedGreenSafe => "Red-Green Safe",
            Self::BlueYellowSafe => "Blue-Yellow Safe",
            Self::Monochrome => "Monochrome",
        }
    }

    pub(crate) fn short(self) -> &'static str {
        match self {
            Self::Standard => "Std",
            Self::RedGreenSafe => "RG",
            Self::BlueYellowSafe => "BY",
            Self::Monochrome => "Mono",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextScalePreset {
    Compact,
    Normal,
    Large,
}

impl TextScalePreset {
    pub(crate) const ALL: [TextScalePreset; 3] = [Self::Compact, Self::Normal, Self::Large];

    pub(crate) fn factor(self) -> f32 {
        match self {
            Self::Compact => 0.92,
            Self::Normal => 1.0,
            Self::Large => 1.12,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Compact => "Compact",
            Self::Normal => "Default",
            Self::Large => "Large",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct AccessibilityConfig {
    pub theme: ColorTheme,
    pub vision: ColorVision,
    pub brightness_pct: u16,
    pub text_scale: TextScalePreset,
    pub glove_targets: bool,
    pub reduced_motion: bool,
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
    pub(crate) const BRIGHTNESS_MIN: u16 = 25;
    pub(crate) const BRIGHTNESS_MAX: u16 = 175;

    pub(crate) fn summary(self) -> String {
        format!(
            "{} · {}% · {}",
            self.theme.label(),
            self.brightness_pct,
            self.vision.short()
        )
    }
}

#[derive(Clone, Copy)]
pub(crate) struct CardColors {
    pub bg: Color32,
    pub hover: Color32,
    pub border: Color32,
    pub accent: Color32,
}

#[derive(Clone, Copy)]
pub(crate) struct Tokens {
    pub outer_bg: Color32,
    pub screen_bg: Color32,
    pub raised_bg: Color32,
    pub raised_hover: Color32,
    pub hairline: Color32,
    pub hairline_strong: Color32,
    pub accent_view: Color32,
    pub accent_input: Color32,
    pub nominal: Color32,
    pub caution: Color32,
    pub critical: Color32,
    pub alert_card: CardColors,
    pub edit_card: CardColors,
    pub text_main: Color32,
    pub text_body: Color32,
    pub text_muted: Color32,
    pub ui_scale: f32,
    pub text_mult: f32,
    pub stroke_w: f32,
    pub gap: f32,
    pub button_h: f32,
    pub anim_secs: f32,
    pub brightness: f32,
    pub theme: ColorTheme,
    pub vision: ColorVision,
}

impl Tokens {
    pub(crate) fn resolve(cfg: AccessibilityConfig, screen_rect: Rect) -> Self {
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

    pub(crate) fn sp(&self, base: f32) -> f32 {
        (base * self.ui_scale).round()
    }

    pub(crate) fn sz(&self, base: f32) -> f32 {
        (base * self.text_mult).clamp(9.0, 64.0)
    }

    pub(crate) fn card_colors(&self, panel: PanelId) -> CardColors {
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

pub(crate) fn scale_brightness(c: Color32, factor: f32) -> Color32 {
    let f = factor.clamp(0.25, 1.75);
    let ch = |v: u8| -> u8 { (v as f32 * f).clamp(0.0, 255.0).round() as u8 };
    Color32::from_rgba_unmultiplied(ch(c.r()), ch(c.g()), ch(c.b()), c.a())
}

pub(crate) fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| -> u8 { (x as f32 + (y as f32 - x as f32) * t).round() as u8 };
    Color32::from_rgba_unmultiplied(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()), mix(a.a(), b.a()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::Pos2;
    use eframe::egui::Vec2;

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
    fn full_color_cards_have_distinct_background_hues() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(960.0, 540.0));
        let t = Tokens::resolve(AccessibilityConfig::default(), rect);
        let c1 = t.card_colors(PanelId::Health).bg;
        let c2 = t.card_colors(PanelId::Hub).bg;
        let c3 = t.card_colors(PanelId::Tech).bg;
        let c4 = t.card_colors(PanelId::Hazard).bg;
        assert_ne!(c1, c2);
        assert_ne!(c2, c3);
        assert_ne!(c3, c4);
        assert_ne!(c1, c4);
    }
}
