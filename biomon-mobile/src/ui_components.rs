//! AEGIS-BIO UI Component Kit (`biomon-mobile` — Astronaut Suit Arm Tablet).
//!
//! Utilizes every component and primitive from `ui_components.rs` while strictly enforcing
//! `DESIGN RULES TO FOLLOW.md`:
//! - `#F0F0E4` (`240, 240, 228`) primary daylight background (zero dark-mode fills)
//! - `#0B2027` (`11, 32, 39`) primary ink/letters
//! - `#4CB944` (`76, 185, 68`) View Mode outline
//! - `#FF8929` (`255, 137, 41`) Input Mode outline
//! - `#CD0929` (`205, 9, 41`) Danger / Alert override
//! - Minimum `26.0 pt` typography on mobile, `5.0 px` / `7.0 px` borders, no sliders/scrollbars.

use eframe::egui::{
    self, Align2, Color32, CornerRadius, CursorIcon, FontData, FontDefinitions, FontFamily, FontId,
    Painter, Pos2, Rect, RichText, Sense, Shape, Stroke, StrokeKind, Ui, Vec2,
};

pub mod palette {
    use eframe::egui::Color32;

    pub const BG_PRIMARY: Color32 = Color32::from_rgb(240, 240, 228); // #F0F0E4
    pub const BG_HOVER: Color32 = Color32::from_rgb(228, 228, 212);
    pub const BG_ACTIVE: Color32 = Color32::from_rgb(214, 214, 196);
    pub const TRACK_BG: Color32 = Color32::from_rgb(222, 222, 206);

    pub const INK_PRIMARY: Color32 = Color32::from_rgb(11, 32, 39); // #0B2027
    pub const OUTLINE_VIEW: Color32 = Color32::from_rgb(76, 185, 68); // #4CB944
    pub const OUTLINE_INPUT: Color32 = Color32::from_rgb(255, 137, 41); // #FF8929
    pub const OUTLINE_ALERT: Color32 = Color32::from_rgb(205, 9, 41); // #CD0929

    pub const NOMINAL: Color32 = OUTLINE_VIEW;
    pub const CAUTION: Color32 = OUTLINE_INPUT;
    pub const CRITICAL: Color32 = OUTLINE_ALERT;
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    Nominal,
    Caution,
    Critical,
}

#[derive(Clone, Copy)]
pub struct CardTheme {
    pub bg: Color32,
    pub hover: Color32,
    pub border: Color32,
    pub accent: Color32,
    pub stroke_width: f32,
}

impl CardTheme {
    pub const VIEW: Self = Self {
        bg: palette::BG_PRIMARY,
        hover: palette::BG_HOVER,
        border: palette::OUTLINE_VIEW,
        accent: palette::OUTLINE_VIEW,
        stroke_width: 5.0,
    };

    pub const INPUT: Self = Self {
        bg: palette::BG_PRIMARY,
        hover: palette::BG_HOVER,
        border: palette::OUTLINE_INPUT,
        accent: palette::OUTLINE_INPUT,
        stroke_width: 5.0,
    };

    pub const ALERT: Self = Self {
        bg: palette::BG_PRIMARY,
        hover: palette::BG_HOVER,
        border: palette::OUTLINE_ALERT,
        accent: palette::OUTLINE_ALERT,
        stroke_width: 7.0,
    };

    pub fn for_mobile(is_input: bool, is_alert: bool) -> Self {
        if is_alert {
            Self::ALERT
        } else if is_input {
            Self::INPUT
        } else {
            Self::VIEW
        }
    }
}

pub fn install_aerospace_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let try_load = |path: &str| std::fs::read(path).ok().map(FontData::from_owned);

    if let Some(data) = try_load("C:\\Windows\\Fonts\\seguisb.ttf")
        .or_else(|| try_load("C:\\Windows\\Fonts\\segoeui.ttf"))
    {
        fonts.font_data.insert("segoe_sb".into(), data.into());
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "segoe_sb".into());
        fonts
            .families
            .entry(FontFamily::Name("heading".into()))
            .or_default()
            .insert(0, "segoe_sb".into());
    }
    if let Some(data) = try_load("C:\\Windows\\Fonts\\CascadiaMono.ttf")
        .or_else(|| try_load("C:\\Windows\\Fonts\\consola.ttf"))
    {
        fonts.font_data.insert("aero_mono".into(), data.into());
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, "aero_mono".into());
    }
    ctx.set_fonts(fonts);
}

pub fn heading_font(size: f32) -> FontId {
    FontId::new(size.max(26.0), FontFamily::Proportional)
}

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| -> u8 { (x as f32 + (y as f32 - x as f32) * t).round() as u8 };
    Color32::from_rgba_unmultiplied(
        mix(a.r(), b.r()),
        mix(a.g(), b.g()),
        mix(a.b(), b.b()),
        mix(a.a(), b.a()),
    )
}

/// Geometry-scaled KPI/stat card for mobile detail views (`>= 26.0 pt` text, zero subtext).
pub fn stat_card(
    ui: &mut Ui,
    height: f32,
    label: &str,
    value: &str,
    val_color: Color32,
    theme: CardTheme,
) {
    let h = height.max(104.0);
    let rect = ui
        .allocate_exact_size(
            Vec2::new(ui.available_width().max(160.0), h),
            Sense::hover(),
        )
        .0;
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(10),
        theme.bg,
        Stroke::new(theme.stroke_width, theme.border),
        StrokeKind::Inside,
    );

    let inner = rect.shrink2(Vec2::new(16.0, 10.0));
    let lbl_sz = 26.0_f32;
    let val_sz = (h * 0.42).clamp(36.0, 56.0);

    p.text(
        inner.left_top(),
        Align2::LEFT_TOP,
        label,
        FontId::new(lbl_sz, FontFamily::Proportional),
        palette::INK_PRIMARY,
    );
    p.text(
        Pos2::new(inner.left(), inner.bottom()),
        Align2::LEFT_BOTTOM,
        value,
        FontId::new(val_sz, FontFamily::Monospace),
        val_color,
    );
}

/// Glove-friendly responsive single-tap button (`min_size >= 140x100`, text `>= 26.0 pt`).
#[allow(clippy::too_many_arguments)]
pub fn touch_button(
    ui: &mut Ui,
    ctx: &egui::Context,
    anim_key: impl std::hash::Hash + std::fmt::Debug,
    min_size: Vec2,
    left: &str,
    right: &str,
    selected: bool,
    theme: CardTheme,
) -> egui::Response {
    let desired = Vec2::new(min_size.x.max(140.0), min_size.y.max(100.0));
    let (rect, resp) = ui.allocate_exact_size(desired, Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    let anim_id = ui.make_persistent_id(anim_key);
    let h_t = ctx.animate_bool_with_time(anim_id, resp.hovered(), 0.08);
    let bg = if selected {
        palette::BG_ACTIVE
    } else {
        lerp_color(theme.bg, theme.hover, h_t)
    };
    let border = if selected {
        palette::OUTLINE_INPUT
    } else {
        lerp_color(theme.border, theme.accent, h_t * 0.7)
    };

    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(10),
        bg,
        Stroke::new(theme.stroke_width, border),
        StrokeKind::Inside,
    );

    let inset = 18.0;
    if right.is_empty() {
        p.text(
            rect.center(),
            Align2::CENTER_CENTER,
            left,
            FontId::new(28.0, FontFamily::Proportional),
            palette::INK_PRIMARY,
        );
    } else {
        p.text(
            Pos2::new(rect.left() + inset, rect.center().y),
            Align2::LEFT_CENTER,
            left,
            FontId::new(28.0, FontFamily::Proportional),
            palette::INK_PRIMARY,
        );
        p.text(
            Pos2::new(rect.right() - inset, rect.center().y),
            Align2::RIGHT_CENTER,
            right,
            FontId::new(26.0, FontFamily::Monospace),
            palette::INK_PRIMARY,
        );
    }
    resp
}

/// Redundant geometric status indicator (Circle / Triangle / Square) for colorblind safety (`>= 28.0 pt` on mobile).
pub fn status_badge(ui: &mut Ui, level: StatusLevel, label: &str) {
    let col = match level {
        StatusLevel::Nominal => palette::NOMINAL,
        StatusLevel::Caution => palette::CAUTION,
        StatusLevel::Critical => palette::CRITICAL,
    };
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover());
        let c = r.center();
        let p = ui.painter();
        match level {
            StatusLevel::Nominal => {
                p.circle_filled(c, 10.0, col);
                p.circle_stroke(c, 10.0, Stroke::new(2.5, palette::INK_PRIMARY));
            }
            StatusLevel::Caution => {
                let pts = vec![
                    Pos2::new(c.x, c.y - 11.0),
                    Pos2::new(c.x + 11.0, c.y + 9.0),
                    Pos2::new(c.x - 11.0, c.y + 9.0),
                ];
                p.add(Shape::convex_polygon(
                    pts,
                    col,
                    Stroke::new(2.5, palette::INK_PRIMARY),
                ));
            }
            StatusLevel::Critical => {
                let sq = Rect::from_center_size(c, Vec2::splat(20.0));
                p.rect(
                    sq,
                    CornerRadius::same(3),
                    col,
                    Stroke::new(2.5, palette::INK_PRIMARY),
                    StrokeKind::Inside,
                );
            }
        }
        ui.label(
            RichText::new(label)
                .font(heading_font(30.0))
                .strong()
                .color(palette::INK_PRIMARY),
        );
    });
}

/// Safe corridor bar (e.g. heart rate within phase min/max) drawn directly via `Painter` or `Ui`.
/// Maps `[lo - 0.28125 * span, hi + 0.28125 * span]` across the full track so `[lo, hi]` aligns
/// with the middle `64%` (`18%..82%`) green safe band and out-of-corridor values visibly exit it.
pub fn paint_corridor_bar(
    p: &Painter,
    rect: Rect,
    val: f32,
    lo: f32,
    hi: f32,
    marker_col: Color32,
) {
    let track = Rect::from_min_size(
        Pos2::new(rect.left(), rect.center().y - 6.0),
        Vec2::new(rect.width(), 12.0),
    );
    p.rect(
        track,
        CornerRadius::same(6),
        palette::TRACK_BG,
        Stroke::new(2.0, palette::INK_PRIMARY),
        StrokeKind::Inside,
    );
    let safe_rect = Rect::from_min_size(
        Pos2::new(track.left() + rect.width() * 0.18, track.top() + 2.0),
        Vec2::new(rect.width() * 0.64, 8.0),
    );
    p.rect_filled(safe_rect, CornerRadius::same(4), palette::OUTLINE_VIEW);

    let span = (hi - lo).max(1.0);
    let wing = span * (0.18 / 0.64);
    let full_lo = lo - wing;
    let full_hi = hi + wing;
    let norm = ((val - full_lo) / (full_hi - full_lo).max(1.0)).clamp(0.03, 0.97);
    let marker_x = track.left() + track.width() * norm;
    let marker_pos = Pos2::new(marker_x, track.center().y);
    let out_of_corridor = val < lo || val > hi;
    let effective_col = if out_of_corridor {
        palette::OUTLINE_ALERT
    } else {
        marker_col
    };
    p.circle_filled(marker_pos, 9.0, effective_col);
    p.circle_stroke(marker_pos, 9.0, Stroke::new(2.5, palette::INK_PRIMARY));
}

pub fn corridor_bar(ui: &mut Ui, width: f32, val: f32, lo: f32, hi: f32, marker_col: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 26.0), Sense::hover());
    paint_corridor_bar(ui.painter(), rect, val, lo, hi, marker_col);
}

/// 4-bar telemetry signal strength icon (Painter primitive + Ui wrapper).
pub fn paint_signal_bars(p: &Painter, rect: Rect, online: bool, alert: bool) {
    let size = rect.width().min(rect.height());
    let s = size / 30.0;
    let active = if online { 4 } else { 1 };
    let col = if alert || !online {
        palette::CRITICAL
    } else {
        palette::INK_PRIMARY
    };
    for i in 0..4 {
        let h = (6.0 + (i as f32) * 5.0) * s;
        let x = rect.left() + 2.0 * s + (i as f32) * 7.0 * s;
        let bar = Rect::from_min_size(
            Pos2::new(x, rect.bottom() - 3.0 * s - h),
            Vec2::new(4.5 * s, h),
        );
        if i < active {
            p.rect_filled(bar, CornerRadius::same(2), col);
        } else {
            p.rect_stroke(
                bar,
                CornerRadius::same(2),
                Stroke::new(2.0, palette::INK_PRIMARY),
                StrokeKind::Inside,
            );
        }
    }
}

pub fn signal_bars(ui: &mut Ui, size: f32, online: bool) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    paint_signal_bars(ui.painter(), rect, online, !online);
}

/// Rotating vector compass dial (Painter primitive + Ui wrapper, zero text < 26pt on mobile).
pub fn paint_compass_dial(
    p: &Painter,
    rect: Rect,
    heading_deg: f32,
    target_deg: f32,
    color: Color32,
) {
    let size = rect.width().min(rect.height());
    let s = size / 68.0;
    let c = rect.center();
    let r = size * 0.44;

    p.circle(
        c,
        r,
        palette::BG_PRIMARY,
        Stroke::new((2.4 * s).max(2.5), palette::INK_PRIMARY),
    );
    for k in 0..12 {
        let ang = (k as f32) * (std::f32::consts::TAU / 12.0);
        let d = Vec2::new(ang.sin(), -ang.cos());
        let len = if k % 3 == 0 { 6.5 * s } else { 3.2 * s };
        let tick_col = if k == 0 {
            palette::OUTLINE_INPUT
        } else {
            palette::INK_PRIMARY
        };
        p.line_segment(
            [c + d * (r - len), c + d * r],
            Stroke::new((1.8 * s).max(2.0), tick_col),
        );
    }

    let rel = (target_deg - heading_deg).to_radians();
    let dir = Vec2::new(rel.sin(), -rel.cos());
    let tip = c + dir * (r * 0.72);
    let perp = Vec2::new(-dir.y, dir.x);
    p.line_segment(
        [c - dir * (r * 0.25), tip],
        Stroke::new((2.8 * s).max(2.5), color),
    );
    p.add(Shape::convex_polygon(
        vec![
            tip,
            tip - dir * (8.0 * s) + perp * (5.0 * s),
            tip - dir * (8.0 * s) - perp * (5.0 * s),
        ],
        color,
        Stroke::new(1.5, palette::INK_PRIMARY),
    ));
    p.circle_filled(c, (3.0 * s).max(2.5), palette::INK_PRIMARY);
}

pub fn compass_dial(ui: &mut Ui, size: f32, heading_deg: f32, target_deg: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    paint_compass_dial(ui.painter(), rect, heading_deg, target_deg, color);
}

/// 5-zone procedural vector spacesuit diagram (`[helmet, torso, scrubber, gloves, shield]`).
pub fn paint_suit_schematic(p: &Painter, rect: Rect, zone_colors: [Color32; 5]) {
    let height = rect.height();
    let s = height / 130.0;
    let cx = rect.center().x;
    let top = rect.top() + 8.0 * s;

    p.circle_stroke(
        rect.center(),
        54.0 * s,
        Stroke::new((2.0 * s).max(2.0), zone_colors[4]),
    );
    let pack = Rect::from_min_size(
        Pos2::new(cx - 28.0 * s, top + 34.0 * s),
        Vec2::new(10.0 * s, 38.0 * s),
    );
    p.rect(
        pack,
        CornerRadius::same(3),
        palette::BG_HOVER,
        Stroke::new((2.0 * s).max(2.0), zone_colors[2]),
        StrokeKind::Inside,
    );

    let head_c = Pos2::new(cx, top + 17.0 * s);
    p.circle(
        head_c,
        15.0 * s,
        palette::BG_PRIMARY,
        Stroke::new((2.2 * s).max(2.0), zone_colors[0]),
    );
    p.rect_filled(
        Rect::from_center_size(
            Pos2::new(cx + 2.0 * s, head_c.y),
            Vec2::new(16.0 * s, 9.5 * s),
        ),
        CornerRadius::same(3),
        zone_colors[0],
    );

    let torso = Rect::from_min_size(
        Pos2::new(cx - 16.0 * s, top + 35.0 * s),
        Vec2::new(32.0 * s, 40.0 * s),
    );
    p.rect(
        torso,
        CornerRadius::same(6),
        palette::BG_HOVER,
        Stroke::new((2.2 * s).max(2.0), zone_colors[1]),
        StrokeKind::Inside,
    );
    p.circle_filled(torso.center(), (4.5 * s).max(3.0), zone_colors[1]);

    for sign in [-1.0_f32, 1.0_f32] {
        let shoulder = Pos2::new(
            if sign < 0.0 {
                torso.left()
            } else {
                torso.right()
            },
            torso.top() + 5.0 * s,
        );
        let hand = Pos2::new(shoulder.x + sign * 12.0 * s, torso.top() + 30.0 * s);
        p.line_segment(
            [shoulder, hand],
            Stroke::new((4.2 * s).max(2.5), zone_colors[1]),
        );
        p.circle_filled(hand, (4.5 * s).max(3.0), zone_colors[3]);
        p.line_segment(
            [
                Pos2::new(cx + sign * 8.0 * s, torso.bottom()),
                Pos2::new(cx + sign * 10.0 * s, torso.bottom() + 34.0 * s),
            ],
            Stroke::new((5.2 * s).max(3.0), zone_colors[1]),
        );
    }
}

pub fn suit_schematic(ui: &mut Ui, height: f32, zone_colors: [Color32; 5]) {
    let s = height / 130.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(115.0 * s, height), Sense::hover());
    paint_suit_schematic(ui.painter(), rect, zone_colors);
}
