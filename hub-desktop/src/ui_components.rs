//! AEGIS-BIO UI Component Kit (`hub-desktop` — Habitat & Flight Surgeon Terminal).
//!
//! Kutup light theme selected by the user; existing component geometry is preserved.
//! - `#F7F8F3` soft white surface; `#E6E9E5` outer background
//! - `#173B3B` dark green text; `#187E78` teal accents
//! - `#719A93` quiet panel borders; `#1A7862` nominal status
//! - `#A65C16` amber input/caution state
//! - `#CD0929` (`205, 9, 41`) Danger / Alert override
//! - Desktop borders `2.5 px` to `3.5 px`, headers `22.0-28.0 pt`, data labels `15.0-18.0 pt`.

use eframe::egui::{
    self, Align2, Color32, CornerRadius, CursorIcon, FontData, FontDefinitions, FontFamily, FontId,
    Pos2, Rect, RichText, Sense, Shape, Stroke, StrokeKind, Ui, Vec2,
};

pub mod palette {
    use eframe::egui::Color32;

    pub const BG_PRIMARY: Color32 = Color32::from_rgb(247, 248, 243); // #F7F8F3
    pub const BG_HOVER: Color32 = Color32::from_rgb(233, 240, 234);
    pub const BG_ACTIVE: Color32 = Color32::from_rgb(217, 229, 223);
    pub const TRACK_BG: Color32 = Color32::from_rgb(217, 229, 223);

    pub const INK_PRIMARY: Color32 = Color32::from_rgb(23, 59, 59); // #173B3B
    pub const OUTLINE_VIEW: Color32 = Color32::from_rgb(24, 126, 120); // #187E78
    pub const OUTLINE_INPUT: Color32 = Color32::from_rgb(166, 92, 22); // #A65C16
    pub const OUTLINE_ALERT: Color32 = Color32::from_rgb(205, 9, 41); // #CD0929

    pub const BG_OUTER: Color32 = Color32::from_rgb(230, 233, 229); // #E6E9E5
    pub const BORDER_VIEW: Color32 = Color32::from_rgb(113, 154, 147); // #719A93
    pub const INK_SECONDARY: Color32 = Color32::from_rgb(83, 106, 105); // #536A69
    pub const NOMINAL: Color32 = Color32::from_rgb(26, 120, 98); // #1A7862
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
}

impl CardTheme {
    pub const HEALTH: Self = Self {
        bg: palette::BG_PRIMARY,
        hover: palette::BG_HOVER,
        border: palette::BORDER_VIEW,
        accent: palette::OUTLINE_VIEW,
    };
    pub const HUB: Self = Self {
        bg: palette::BG_PRIMARY,
        hover: palette::BG_HOVER,
        border: palette::BORDER_VIEW,
        accent: palette::OUTLINE_VIEW,
    };
    pub const COMMS: Self = Self {
        bg: palette::BG_PRIMARY,
        hover: palette::BG_HOVER,
        border: palette::BORDER_VIEW,
        accent: palette::OUTLINE_VIEW,
    };
    pub const HAZARD: Self = Self {
        bg: palette::BG_PRIMARY,
        hover: palette::BG_HOVER,
        border: palette::BORDER_VIEW,
        accent: palette::OUTLINE_VIEW,
    };
    pub const INPUT: Self = Self {
        bg: palette::BG_PRIMARY,
        hover: palette::BG_HOVER,
        border: palette::OUTLINE_INPUT,
        accent: palette::OUTLINE_INPUT,
    };
    pub const ALERT: Self = Self {
        bg: palette::BG_PRIMARY,
        hover: palette::BG_HOVER,
        border: palette::OUTLINE_ALERT,
        accent: palette::OUTLINE_ALERT,
    };
}

pub fn install_aerospace_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let try_load = |path: &str| std::fs::read(path).ok().map(FontData::from_owned);

    if let Some(data) = try_load("C:\\Windows\\Fonts\\segoeui.ttf") {
        fonts.font_data.insert("segoe_ui".into(), data.into());
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "segoe_ui".into());
    }
    if let Some(data) = try_load("C:\\Windows\\Fonts\\seguisb.ttf") {
        fonts.font_data.insert("segoe_sb".into(), data.into());
        fonts
            .families
            .entry(FontFamily::Name("heading".into()))
            .or_default()
            .push("segoe_sb".into());
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
    FontId::new(size, FontFamily::Proportional)
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

/// Geometry-scaled KPI/stat card with monospace value and bottom note (Desktop `3.0 px` border).
pub fn stat_card(
    ui: &mut Ui,
    height: f32,
    label: &str,
    value: &str,
    val_color: Color32,
    note: &str,
    theme: CardTheme,
) {
    let rect = ui
        .allocate_exact_size(
            Vec2::new(ui.available_width(), height.max(84.0)),
            Sense::hover(),
        )
        .0;
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(8),
        theme.bg,
        Stroke::new(3.0, theme.border),
        StrokeKind::Inside,
    );

    let pad = Vec2::new(14.0, 10.0);
    let inner = rect.shrink2(pad);
    let has_note = !note.is_empty();

    let lbl_sz = 16.0_f32;
    let note_sz = 15.0_f32;
    let chars = value.chars().count().max(1) as f32;
    let fit_w = inner.width() / (chars * 0.62);
    let val_room = inner.height() - lbl_sz - if has_note { note_sz } else { 0.0 } - 6.0;
    let val_sz = (val_room * 0.88).min(fit_w).clamp(18.0, 32.0);

    p.text(
        inner.left_top(),
        Align2::LEFT_TOP,
        label,
        FontId::proportional(lbl_sz),
        palette::INK_PRIMARY,
    );
    let val_y = inner.top()
        + lbl_sz
        + (inner.height() - lbl_sz - if has_note { note_sz } else { 0.0 }) * 0.5;
    p.text(
        Pos2::new(inner.left(), val_y),
        Align2::LEFT_CENTER,
        value,
        FontId::monospace(val_sz),
        val_color,
    );
    if has_note {
        p.text(
            inner.left_bottom(),
            Align2::LEFT_BOTTOM,
            note,
            FontId::monospace(note_sz),
            palette::INK_PRIMARY,
        );
    }
}

/// Touch-friendly responsive button with left title and right monospace tag (`3.0 px` semantic border).
pub fn touch_button(
    ui: &mut Ui,
    anim_key: impl std::hash::Hash + std::fmt::Debug,
    height: f32,
    left: &str,
    right: &str,
    selected: bool,
    theme: CardTheme,
) -> egui::Response {
    let desired = Vec2::new(ui.available_width(), height.max(38.0));
    let (rect, resp) = ui.allocate_exact_size(desired, Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    let anim_id = ui.make_persistent_id(anim_key);
    let h_t = ui
        .ctx()
        .animate_bool_with_time(anim_id, resp.hovered(), 0.08);
    let (bg, border) = if selected {
        (palette::BG_ACTIVE, palette::OUTLINE_INPUT)
    } else {
        (
            lerp_color(theme.bg, theme.hover, h_t),
            lerp_color(theme.border, theme.accent, h_t * 0.7),
        )
    };

    let inset = (desired.y * 0.24).clamp(10.0, 18.0);
    let left_sz = (desired.y * 0.38).clamp(16.0, 22.0);
    let right_sz = (left_sz * 0.88).clamp(15.0, 18.0);

    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(7),
        bg,
        Stroke::new(3.0, border),
        StrokeKind::Inside,
    );
    p.text(
        Pos2::new(rect.left() + inset, rect.center().y),
        Align2::LEFT_CENTER,
        left,
        FontId::proportional(left_sz),
        palette::INK_PRIMARY,
    );
    if !right.is_empty() {
        p.text(
            Pos2::new(rect.right() - inset, rect.center().y),
            Align2::RIGHT_CENTER,
            right,
            FontId::monospace(right_sz),
            palette::INK_PRIMARY,
        );
    }
    resp
}

/// Desktop progress-bar control (e.g. Habitat Display / Sim Intensity `25%–175%`).
pub fn progress_slider(
    ui: &mut Ui,
    height: f32,
    label: &str,
    value: u16,
    lo: u16,
    hi: u16,
    theme: CardTheme,
) -> Option<u16> {
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), height.max(46.0)),
        Sense::click_and_drag(),
    );
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
    }
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(8),
        theme.bg,
        Stroke::new(3.0, palette::OUTLINE_INPUT),
        StrokeKind::Inside,
    );

    let pad = 6.0;
    let track = rect.shrink(pad);
    let frac = (value.clamp(lo, hi) - lo) as f32 / (hi - lo).max(1) as f32;
    let fill = Rect::from_min_max(
        track.min,
        Pos2::new(track.left() + track.width() * frac, track.bottom()),
    );

    p.rect_filled(track, CornerRadius::same(5), palette::TRACK_BG);
    p.rect_filled(
        fill,
        CornerRadius::same(5),
        palette::OUTLINE_INPUT.gamma_multiply(0.32),
    );
    let hx = track.left() + track.width() * frac;
    p.line_segment(
        [
            Pos2::new(hx, rect.top() + 3.0),
            Pos2::new(hx, rect.bottom() - 3.0),
        ],
        Stroke::new(3.0, palette::INK_PRIMARY),
    );

    p.text(
        Pos2::new(track.left() + 12.0, track.center().y),
        Align2::LEFT_CENTER,
        format!("{label}  {value}%"),
        FontId::monospace(16.0),
        palette::INK_PRIMARY,
    );

    if let Some(pos) = resp.interact_pointer_pos()
        && (resp.dragged() || resp.clicked())
    {
        let f = ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0);
        let snapped = (((lo as f32 + f * (hi - lo) as f32) / 5.0).round() * 5.0) as u16;
        let next = snapped.clamp(lo, hi);
        if next != value {
            return Some(next);
        }
    }
    None
}

/// Redundant geometric status indicator (Circle / Triangle / Square) for colorblind safety.
pub fn status_badge(ui: &mut Ui, level: StatusLevel, label: &str) {
    let col = match level {
        StatusLevel::Nominal => palette::NOMINAL,
        StatusLevel::Caution => palette::CAUTION,
        StatusLevel::Critical => palette::CRITICAL,
    };
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
        let c = r.center();
        let p = ui.painter();
        match level {
            StatusLevel::Nominal => {
                p.circle_filled(c, 6.0, col);
                p.circle_stroke(c, 6.0, Stroke::new(1.5, palette::INK_PRIMARY));
            }
            StatusLevel::Caution => {
                p.add(Shape::convex_polygon(
                    vec![
                        Pos2::new(c.x, c.y - 6.5),
                        Pos2::new(c.x + 6.5, c.y + 5.5),
                        Pos2::new(c.x - 6.5, c.y + 5.5),
                    ],
                    col,
                    Stroke::new(1.5, palette::INK_PRIMARY),
                ));
            }
            StatusLevel::Critical => {
                let sq = Rect::from_center_size(c, Vec2::splat(12.0));
                p.rect(
                    sq,
                    CornerRadius::same(2),
                    col,
                    Stroke::new(1.5, palette::INK_PRIMARY),
                    StrokeKind::Inside,
                );
            }
        }
        ui.label(
            RichText::new(label)
                .font(heading_font(16.0))
                .strong()
                .color(palette::INK_PRIMARY),
        );
    });
}

/// Safe corridor bar (e.g. heart rate within phase min/max).
/// Maps `val` across the extended range `[lo - span*0.35, hi + span*0.35]` so the safe
/// band `[lo, hi]` occupies the central ~59% of the bar and out-of-corridor values
/// visibly move into the outer warning regions.
pub fn corridor_bar(ui: &mut Ui, width: f32, val: f32, lo: f32, hi: f32, marker_col: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 18.0), Sense::hover());
    paint_corridor_bar(ui.painter(), rect, val, lo, hi, marker_col);
}

pub fn paint_corridor_bar(
    p: &egui::Painter,
    rect: Rect,
    val: f32,
    lo: f32,
    hi: f32,
    marker_col: Color32,
) {
    let width = rect.width().max(24.0);
    let track = Rect::from_min_size(
        Pos2::new(rect.left(), rect.center().y - 4.5),
        Vec2::new(width, 9.0),
    );
    p.rect(
        track,
        CornerRadius::same(4),
        palette::TRACK_BG,
        Stroke::new(1.5, palette::INK_PRIMARY),
        StrokeKind::Inside,
    );

    let span = (hi - lo).max(1.0);
    let ext_lo = lo - span * 0.35;
    let ext_hi = hi + span * 0.35;
    let ext_span = (ext_hi - ext_lo).max(1.0);

    let safe_start_frac = ((lo - ext_lo) / ext_span).clamp(0.0, 1.0);
    let safe_end_frac = ((hi - ext_lo) / ext_span).clamp(0.0, 1.0);

    let safe_rect = Rect::from_min_size(
        Pos2::new(track.left() + width * safe_start_frac, track.top() + 1.5),
        Vec2::new(width * (safe_end_frac - safe_start_frac), 6.0),
    );
    p.rect_filled(safe_rect, CornerRadius::same(3), palette::OUTLINE_VIEW);

    let norm = ((val - ext_lo) / ext_span).clamp(0.02, 0.98);
    let marker = Pos2::new(track.left() + width * norm, track.center().y);
    p.circle_filled(marker, 6.5, marker_col);
    p.circle_stroke(marker, 6.5, Stroke::new(1.8, palette::INK_PRIMARY));
}

/// 4-bar telemetry signal strength icon.
pub fn signal_bars(ui: &mut Ui, size: f32, online: bool) {
    let s = size / 30.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let p = ui.painter();
    let active = if online { 4 } else { 1 };
    let col = if online {
        palette::INK_PRIMARY
    } else {
        palette::CRITICAL
    };
    for i in 0..4 {
        let h = (6.0 + (i as f32) * 5.0) * s;
        let x = rect.left() + 2.0 * s + (i as f32) * 7.0 * s;
        let bar = Rect::from_min_size(
            Pos2::new(x, rect.bottom() - 3.0 * s - h),
            Vec2::new(4.5 * s, h),
        );
        if i < active {
            p.rect_filled(bar, CornerRadius::same(1), col);
        } else {
            p.rect_stroke(
                bar,
                CornerRadius::same(1),
                Stroke::new(1.5, palette::INK_PRIMARY),
                StrokeKind::Inside,
            );
        }
    }
}

/// Rotating vector compass dial (`>= 15.0 pt` cardinal labels on Desktop).
pub fn compass_dial(ui: &mut Ui, size: f32, heading_deg: f32, target_deg: f32, color: Color32) {
    let s = size / 68.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let c = rect.center();
    let r = size * 0.44;
    let p = ui.painter();

    p.circle(
        c,
        r,
        palette::BG_PRIMARY,
        Stroke::new(2.5, palette::INK_PRIMARY),
    );
    for k in 0..12 {
        let ang = (k as f32) * (std::f32::consts::TAU / 12.0);
        let d = Vec2::new(ang.sin(), -ang.cos());
        let len = if k % 3 == 0 { 5.5 * s } else { 2.8 * s };
        p.line_segment(
            [c + d * (r - len), c + d * r],
            Stroke::new(1.6, palette::INK_PRIMARY),
        );
    }
    for (lbl, deg) in [("N", 0.0_f32), ("E", 90.0), ("S", 180.0), ("W", 270.0)] {
        let rad = (deg - heading_deg).to_radians();
        let pos = c + Vec2::new(rad.sin(), -rad.cos()) * (r - 12.0 * s);
        let col = if lbl == "N" {
            palette::OUTLINE_INPUT
        } else {
            palette::INK_PRIMARY
        };
        p.text(
            pos,
            Align2::CENTER_CENTER,
            lbl,
            FontId::monospace(15.0),
            col,
        );
    }
    let rel = (target_deg - heading_deg).to_radians();
    let dir = Vec2::new(rel.sin(), -rel.cos());
    let tip = c + dir * (r * 0.72);
    let perp = Vec2::new(-dir.y, dir.x);
    p.line_segment([c - dir * (r * 0.25), tip], Stroke::new(2.4 * s, color));
    p.add(Shape::convex_polygon(
        vec![
            tip,
            tip - dir * (6.5 * s) + perp * (4.5 * s),
            tip - dir * (6.5 * s) - perp * (4.5 * s),
        ],
        color,
        Stroke::new(1.5, palette::INK_PRIMARY),
    ));
    p.circle_filled(c, 3.0 * s, palette::INK_PRIMARY);
}

/// 5-zone procedural vector spacesuit diagram (`[helmet, torso, scrubber, gloves, shield]`).
pub fn suit_schematic(ui: &mut Ui, height: f32, zone_colors: [Color32; 5]) {
    let s = height / 130.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(115.0 * s, height), Sense::hover());
    let p = ui.painter();
    let cx = rect.center().x;
    let top = rect.top() + 8.0 * s;

    p.circle_stroke(rect.center(), 56.0 * s, Stroke::new(2.0, zone_colors[4]));
    let pack = Rect::from_min_size(
        Pos2::new(cx - 28.0 * s, top + 34.0 * s),
        Vec2::new(10.0 * s, 38.0 * s),
    );
    p.rect(
        pack,
        CornerRadius::same(3),
        palette::BG_HOVER,
        Stroke::new(2.0, zone_colors[2]),
        StrokeKind::Inside,
    );

    let head_c = Pos2::new(cx, top + 17.0 * s);
    p.circle(
        head_c,
        15.0 * s,
        palette::BG_PRIMARY,
        Stroke::new(2.2, zone_colors[0]),
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
        Stroke::new(2.2, zone_colors[1]),
        StrokeKind::Inside,
    );
    p.circle_filled(torso.center(), 4.5 * s, zone_colors[1]);

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
        p.line_segment([shoulder, hand], Stroke::new(4.2 * s, zone_colors[1]));
        p.circle_filled(hand, 4.5 * s, zone_colors[3]);
        p.line_segment(
            [
                Pos2::new(cx + sign * 8.0 * s, torso.bottom()),
                Pos2::new(cx + sign * 10.0 * s, torso.bottom() + 34.0 * s),
            ],
            Stroke::new(5.2 * s, zone_colors[1]),
        );
    }
}
