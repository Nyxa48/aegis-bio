//! Standalone AEGIS-BIO UI Component Kit & Live Showcase.
//!
//!Strictly follows `DESIGN RULES TO FOLLOW.md`:
//! - `#F0F0E4` (`240, 240, 228`) primary daylight background (zero dark-mode fills)
//! - `#0B2027` (`11, 32, 39`) primary ink/letters
//! - `#4CB944` (`76, 185, 68`) View Mode outline
//! - `#FF8929` (`255, 137, 41`) Input Mode outline
//! - `#CD0929` (`205, 9, 41`) Danger / Alert override

use eframe::egui::{
    self, Align2, Color32, CornerRadius, CursorIcon, FontData, FontDefinitions, FontFamily, FontId,
    Frame, Margin, Pos2, Rect, RichText, Sense, Shape, Stroke, StrokeKind, Ui, Vec2,
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
}

impl CardTheme {
    pub const HEALTH: Self = Self {
        bg: palette::BG_PRIMARY,
        hover: palette::BG_HOVER,
        border: palette::OUTLINE_VIEW,
        accent: palette::OUTLINE_VIEW,
    };
    pub const HUB: Self = Self {
        bg: palette::BG_PRIMARY,
        hover: palette::BG_HOVER,
        border: palette::OUTLINE_VIEW,
        accent: palette::OUTLINE_VIEW,
    };
    pub const COMMS: Self = Self {
        bg: palette::BG_PRIMARY,
        hover: palette::BG_HOVER,
        border: palette::OUTLINE_VIEW,
        accent: palette::OUTLINE_VIEW,
    };
    pub const HAZARD: Self = Self {
        bg: palette::BG_PRIMARY,
        hover: palette::BG_HOVER,
        border: palette::OUTLINE_VIEW,
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

/// Geometry-scaled KPI/stat card with monospace value and optional bottom note.
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
    let val_y =
        inner.top() + lbl_sz + (inner.height() - lbl_sz - if has_note { note_sz } else { 0.0 }) * 0.5;
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

/// Touch-friendly responsive button with left title and right monospace tag.
pub fn touch_button(
    ui: &mut Ui,
    ctx: &egui::Context,
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
    let h_t = ctx.animate_bool_with_time(anim_idanim_id, resp.hovered(), 0.08);
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

/// Touch/drag progress-bar slider (e.g. brightness 25%–175%). Returns `Some(new_val)` on change.
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
pub fn corridor_bar(ui: &mut Ui, width: f32, val: f32, lo: f32, hi: f32, marker_col: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 16.0), Sense::hover());
    let p = ui.painter();
    let track = Rect::from_min_size(
        Pos2::new(rect.left(), rect.center().y - 4.0),
        Vec2::new(width, 8.0),
    );
    p.rect(
        track,
        CornerRadius::same(4),
        palette::TRACK_BG,
        Stroke::new(1.5, palette::INK_PRIMARY),
        StrokeKind::Inside,
    );
    let safe_rect = Rect::from_min_size(
        Pos2::new(track.left() + width * 0.2, track.top() + 1.5),
        Vec2::new(width * 0.6, 5.0),
    );
    p.rect_filled(safe_rect, CornerRadius::same(3), palette::OUTLINE_VIEW);
    let norm = ((val - lo) / (hi - lo).max(1.0)).clamp(0.0, 1.0);
    let marker = Pos2::new(safe_rect.left() + safe_rect.width() * norm, track.center().y);
    p.circle_filled(marker, 6.0, marker_col);
    p.circle_stroke(marker, 6.0, Stroke::new(1.8, palette::INK_PRIMARY));
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

/// Rotating vector compass dial (`>= 15.0 pt` cardinal labels).
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

    p.circle_stroke(
        rect.center(),
        56.0 * s,
        Stroke::new(2.0, zone_colors[4]),
    );
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

// ───────────────────────── Live Interactive Showcase ─────────────────────────
struct ShowcaseApp {
    brightness: u16,
    selected_idx: usize,
    heading: f32,
}

impl Default for ShowcaseApp {
    fn default() -> Self {
        Self {
            brightness: 100,
            selected_idx: 1,
            heading: 42.0,
        }
    }
}

impl eframe::App for ShowcaseApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(palette::BG_PRIMARY).inner_margin(Margin::same(14)))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("AEGIS-BIO // UI COMPONENT SHOWCASE")
                        .font(heading_font(24.0))
                        .strong()
                        .color(palette::INK_PRIMARY),
                );
                ui.add_space(10.0);

                ui.columns(4, |c| {
                    stat_card(
                        &mut c[0],
                        96.0,
                        "HEART RATE",
                        "88 BPM",
                        palette::INK_PRIMARY,
                        "45–100 BPM",
                        CardTheme::HEALTH,
                    );
                    stat_card(
                        &mut c[1],
                        96.0,
                        "TARGET DISTANCE",
                        "222 M",
                        palette::INK_PRIMARY,
                        "HABITAT",
                        CardTheme::HUB,
                    );
                    stat_card(
                        &mut c[2],
                        96.0,
                        "SURFACE LAN",
                        "ONLINE",
                        palette::INK_PRIMARY,
                        "127.0.0.1:7878",
                        CardTheme::COMMS,
                    );
                    stat_card(
                        &mut c[3],
                        96.0,
                        "RADIATION",
                        "0.42 uSv/h",
                        palette::INK_PRIMARY,
                        "LIMIT < 50.0",
                        CardTheme::HAZARD,
                    );
                });

                ui.add_space(12.0);
                ui.columns(2, |cols| {
                    cols[0].vertical(|ui| {
                        if let Some(v) = progress_slider(
                            ui,
                            56.0,
                            "GAIN",
                            self.brightness,
                            25,
                            175,
                            CardTheme::INPUT,
                        ) {
                            self.brightness = v;
                        }
                        ui.add_space(8.0);
                        for (i, label) in ["LAUNCH", "ORBIT", "EVA", "REENTRY"].iter().enumerate()
                        {
                            if touch_button(
                                ui,
                                &ctx,
                                ("btn", i),
                                44.0,
                                label,
                                if self.selected_idx == i { "ACTIVE" } else { "" },
                                self.selected_idx == i,
                                CardTheme::INPUT,
                            )
                            .clicked()
                            {
                                self.selected_idx = i;
                            }
                            ui.add_space(6.0);
                        }
                    });

                    cols[1].vertical(|ui| {
                        Frame::NONE
                            .fill(palette::BG_PRIMARY)
                            .stroke(Stroke::new(3.0, palette::OUTLINE_VIEW))
                            .corner_radius(CornerRadius::same(8))
                            .inner_margin(Margin::same(12))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    status_badge(ui, StatusLevel::Nominal, "NOMINAL");
                                    status_badge(ui, StatusLevel::Caution, "CAUTION");
                                    status_badge(ui, StatusLevel::Critical, "CRITICAL");
                                    signal_bars(ui, 24.0, true);
                                });
                                ui.add_space(8.0);
                                let col_w = ui.available_width();
                                corridor_bar(
                                    ui,
                                    col_wcol_w,
                                    88.0,
                                    45.0,
                                    100.0,
                                    palette::OUTLINE_VIEW,
                                );
                                ui.add_space(10.0);
                                ui.horizontal(|ui| {
                                    compass_dial(
                                        ui,
                                        120.0,
                                        self.heading,
                                        42.0,
                                        palette::OUTLINE_INPUT,
                                    );
                                    ui.add_space(16.0);
                                    suit_schematic(
                                        ui,
                                        120.0,
                                        [
                                            palette::INK_PRIMARY,
                                            palette::NOMINAL,
                                            palette::CAUTION,
                                            palette::INK_PRIMARY,
                                            palette::NOMINAL,
                                        ],
                                    );
                                });
                            });
                    });
                });
            });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 540.0])
            .with_title("AEGIS-BIO // UI Components"),
        ..Default::default()
    };
    eframe::run_native(
        "AEGIS-BIO UI Components",
        options,
        Box::new(|cc| {
            install_aerospace_fonts(&cc.egui_ctx);
            let mut v = egui::Visuals::light();
            v.dark_mode = false;
            v.panel_fill = palette::BG_PRIMARY;
            v.window_fill = palette::BG_PRIMARY;
            v.override_text_color = Some(palette::INK_PRIMARY);
            v.window_shadow = egui::epaint::Shadow::NONE;
            v.popup_shadow = egui::epaint::Shadow::NONE;
            cc.egui_ctx.set_visuals(v);
            Ok(Box::new(ShowcaseApp::default()))
        }),
    )
}
