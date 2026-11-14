//! Standalone AEGIS-BIO UI Component Kit & Live Showcase.
//!
//! Contains only visual design tokens, color palettes, typography setup, and reusable
//! `egui::Painter` UI components (zero mission/network/storage logic) so teammates can
//! inspect or copy the UI design directly, or preview it via:
//! `cargo run --example ui_components`

use eframe::egui::{
    self, Align2, Color32, CursorIcon, FontData, FontDefinitions, FontFamily, FontId, Frame, Id,
    Margin, Pos2, Rect, RichText, Rounding, Sense, Shape, Stroke, Ui, Vec2,
};

pub mod palette {
    use eframe::egui::Color32;

    pub const VOID_950: Color32 = Color32::from_rgb(11, 15, 23);
    pub const SCREEN_900: Color32 = Color32::from_rgb(16, 22, 34);
    pub const SLATE_850: Color32 = Color32::from_rgb(26, 34, 50);
    pub const SLATE_800: Color32 = Color32::from_rgb(34, 44, 64);
    pub const SLATE_700: Color32 = Color32::from_rgb(52, 66, 92);
    pub const SLATE_600: Color32 = Color32::from_rgb(76, 94, 124);

    pub const TEXT_WHITE: Color32 = Color32::from_rgb(248, 250, 252);
    pub const TEXT_BODY: Color32 = Color32::from_rgb(209, 217, 230);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(148, 163, 184);

    pub const NOMINAL: Color32 = Color32::from_rgb(34, 197, 94);
    pub const CAUTION: Color32 = Color32::from_rgb(245, 158, 11);
    pub const CRITICAL: Color32 = Color32::from_rgb(239, 68, 68);

    // Subsystem full-color card themes
    pub const TEAL_BG: Color32 = Color32::from_rgb(13, 36, 33);
    pub const TEAL_HOVER: Color32 = Color32::from_rgb(18, 52, 47);
    pub const TEAL_BORDER: Color32 = Color32::from_rgb(29, 98, 86);
    pub const TEAL_ACCENT: Color32 = Color32::from_rgb(45, 212, 191);

    pub const SKY_BG: Color32 = Color32::from_rgb(13, 31, 51);
    pub const SKY_HOVER: Color32 = Color32::from_rgb(18, 45, 72);
    pub const SKY_BORDER: Color32 = Color32::from_rgb(31, 92, 132);
    pub const SKY_ACCENT: Color32 = Color32::from_rgb(56, 189, 248);

    pub const INDIGO_BG: Color32 = Color32::from_rgb(25, 23, 49);
    pub const INDIGO_HOVER: Color32 = Color32::from_rgb(35, 32, 70);
    pub const INDIGO_BORDER: Color32 = Color32::from_rgb(76, 68, 152);
    pub const INDIGO_ACCENT: Color32 = Color32::from_rgb(167, 139, 250);

    pub const AMBER_BG: Color32 = Color32::from_rgb(38, 27, 13);
    pub const AMBER_HOVER: Color32 = Color32::from_rgb(54, 38, 18);
    pub const AMBER_BORDER: Color32 = Color32::from_rgb(126, 82, 26);
    pub const AMBER_ACCENT: Color32 = Color32::from_rgb(251, 146, 60);
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
        bg: palette::TEAL_BG,
        hover: palette::TEAL_HOVER,
        border: palette::TEAL_BORDER,
        accent: palette::TEAL_ACCENT,
    };
    pub const HUB: Self = Self {
        bg: palette::SKY_BG,
        hover: palette::SKY_HOVER,
        border: palette::SKY_BORDER,
        accent: palette::SKY_ACCENT,
    };
    pub const COMMS: Self = Self {
        bg: palette::INDIGO_BG,
        hover: palette::INDIGO_HOVER,
        border: palette::INDIGO_BORDER,
        accent: palette::INDIGO_ACCENT,
    };
    pub const HAZARD: Self = Self {
        bg: palette::AMBER_BG,
        hover: palette::AMBER_HOVER,
        border: palette::AMBER_BORDER,
        accent: palette::AMBER_ACCENT,
    };
}

pub fn install_aerospace_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let try_load = |path: &str| std::fs::read(path).ok().map(FontData::from_owned);

    if let Some(data) = try_load("C:\\Windows\\Fonts\\segoeui.ttf") {
        fonts.font_data.insert("segoe_ui".into(), data);
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "segoe_ui".into());
    }
    if let Some(data) = try_load("C:\\Windows\\Fonts\\seguisb.ttf") {
        fonts.font_data.insert("segoe_sb".into(), data);
        fonts
            .families
            .entry(FontFamily::Name("heading".into()))
            .or_default()
            .push("segoe_sb".into());
    }
    if let Some(data) = try_load("C:\\Windows\\Fonts\\CascadiaMono.ttf")
        .or_else(|| try_load("C:\\Windows\\Fonts\\consola.ttf"))
    {
        fonts.font_data.insert("aero_mono".into(), data);
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, "aero_mono".into());
    }
    ctx.set_fonts(fonts);
}

pub fn heading_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("heading".into()))
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
        .allocate_exact_size(Vec2::new(ui.available_width(), height.max(40.0)), Sense::hover())
        .0;
    let p = ui.painter();
    p.rect(rect, 8.0, theme.bg, Stroke::new(1.2_f32, theme.border));

    let pad = Vec2::new((rect.width() * 0.06).clamp(8.0, 22.0), (rect.height() * 0.09).clamp(6.0, 18.0));
    let inner = rect.shrink2(pad);
    let has_note = !note.is_empty();

    let lbl_sz = (inner.height() * 0.14).clamp(11.0, 24.0);
    let note_sz = (inner.height() * 0.12).clamp(10.0, 20.0);
    let chars = value.chars().count().max(1) as f32;
    let fit_w = inner.width() / (chars * 0.62);
    let val_room = inner.height() - lbl_sz - if has_note { note_sz } else { 0.0 } - 6.0;
    let val_sz = (val_room * 0.92).min(fit_w).min(val_room).clamp(12.0, 96.0);

    p.text(inner.left_top(), Align2::LEFT_TOP, label, FontId::proportional(lbl_sz), palette::TEXT_MUTED);
    let val_y = inner.top() + lbl_sz + (inner.height() - lbl_sz - if has_note { note_sz } else { 0.0 }) * 0.5;
    p.text(Pos2::new(inner.left(), val_y), Align2::LEFT_CENTER, value, FontId::monospace(val_sz), val_color);
    if has_note {
        p.text(inner.left_bottom(), Align2::LEFT_BOTTOM, note, FontId::monospace(note_sz), palette::TEXT_MUTED);
    }
}

/// Touch-friendly responsive button with left title and right monospace tag.
pub fn touch_button(
    ui: &mut Ui,
    ctx: &egui::Context,
    anim_key: impl std::hash::Hash,
    height: f32,
    left: &str,
    right: &str,
    selected: bool,
    theme: CardTheme,
) -> egui::Response {
    let desired = Vec2::new(ui.available_width(), height.max(28.0));
    let (rect, resp) = ui.allocate_exact_size(desired, Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    let h_t = ctx.animate_bool_with_time(Id::new(anim_key), resp.hovered(), 0.08);
    let (bg, border, text_col) = if selected {
        (Color32::from_rgb(50, 38, 18), palette::CAUTION, palette::CAUTION)
    } else {
        (
            lerp_color(theme.bg, theme.hover, h_t),
            lerp_color(theme.border, theme.accent, h_t * 0.7),
            palette::TEXT_WHITE,
        )
    };

    let inset = (desired.y * 0.24).clamp(8.0, 20.0);
    let avail_w = (desired.x - inset * 2.0).max(40.0);
    let left_chars = left.chars().count().max(1) as f32;
    let right_chars = right.chars().count() as f32;
    let w_factor = if right.is_empty() {
        left_chars * 0.56
    } else {
        left_chars * 0.56 + right_chars * 0.46 + 1.5
    };
    let left_sz = (desired.y * 0.34).min(avail_w / w_factor).clamp(11.5, 34.0);
    let right_sz = (left_sz * 0.72).clamp(9.5, 24.0);

    let p = ui.painter();
    p.rect(rect, 7.0, bg, Stroke::new(1.2_f32, border));
    p.text(Pos2::new(rect.left() + inset, rect.center().y), Align2::LEFT_CENTER, left, FontId::proportional(left_sz), text_col);
    if !right.is_empty() {
        p.text(
            Pos2::new(rect.right() - inset, rect.center().y),
            Align2::RIGHT_CENTER,
            right,
            FontId::monospace(right_sz),
            if selected { palette::CAUTION } else { palette::TEXT_MUTED },
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
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height.max(40.0)), Sense::click_and_drag());
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
    }
    let p = ui.painter();
    p.rect(rect, 8.0, theme.bg, Stroke::new(1.2_f32, theme.border));

    let pad = (rect.height() * 0.14).clamp(5.0, 14.0);
    let track = rect.shrink(pad);
    let frac = (value.clamp(lo, hi) - lo) as f32 / (hi - lo).max(1) as f32;
    let fill = Rect::from_min_max(track.min, Pos2::new(track.left() + track.width() * frac, track.bottom()));

    p.rect_filled(track, 5.0, palette::SLATE_850);
    p.rect_filled(fill, 5.0, theme.accent.gamma_multiply(0.55));
    let hx = track.left() + track.width() * frac;
    p.line_segment([Pos2::new(hx, rect.top() + 2.0), Pos2::new(hx, rect.bottom() - 2.0)], Stroke::new(3.0_f32, palette::TEXT_WHITE));

    let txt_sz = (track.height() * 0.34).min(track.width() * 0.068).clamp(12.0, 32.0);
    p.text(
        Pos2::new(track.left() + pad, track.center().y),
        Align2::LEFT_CENTER,
        format!("{label}  {value}%"),
        FontId::monospace(txt_sz),
        palette::TEXT_WHITE,
    );

    if let Some(pos) = resp.interact_pointer_pos() {
        if resp.dragged() || resp.clicked() {
            let f = ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0);
            let snapped = (((lo as f32 + f * (hi - lo) as f32) / 5.0).round() * 5.0) as u16;
            let next = snapped.clamp(lo, hi);
            if next != value {
                return Some(next);
            }
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
        let (r, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
        let c = r.center();
        let p = ui.painter();
        match level {
            StatusLevel::Nominal => {
                p.circle_filled(c, 4.0, col);
            }
            StatusLevel::Caution => {
                p.add(Shape::convex_polygon(
                    vec![Pos2::new(c.x, c.y - 4.4), Pos2::new(c.x + 4.4, c.y + 3.6), Pos2::new(c.x - 4.4, c.y + 3.6)],
                    col,
                    Stroke::NONE,
                ));
            }
            StatusLevel::Critical => {
                p.rect_filled(Rect::from_center_size(c, Vec2::splat(8.0)), 1.5, col);
            }
        }
        ui.label(RichText::new(label).font(heading_font(12.5)).color(col));
    });
}

/// Safe corridor bar (e.g. heart rate within phase min/max).
pub fn corridor_bar(ui: &mut Ui, width: f32, val: f32, lo: f32, hi: f32, marker_col: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 10.0), Sense::hover());
    let p = ui.painter();
    let track = Rect::from_min_size(Pos2::new(rect.left(), rect.center().y - 2.2), Vec2::new(width, 4.4));
    p.rect_filled(track, 2.2, palette::SLATE_700);
    let safe_rect = Rect::from_min_size(Pos2::new(track.left() + width * 0.2, track.top()), Vec2::new(width * 0.6, 4.4));
    p.rect_filled(safe_rect, 2.2, palette::SLATE_600);
    let norm = ((val - lo) / (hi - lo).max(1.0)).clamp(0.0, 1.0);
    p.circle_filled(Pos2::new(safe_rect.left() + safe_rect.width() * norm, track.center().y), 4.0, marker_col);
}

/// 4-bar telemetry signal strength icon.
pub fn signal_bars(ui: &mut Ui, size: f32, online: bool) {
    let s = size / 30.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let p = ui.painter();
    let active = if online { 4 } else { 1 };
    let col = if online { palette::NOMINAL } else { palette::CAUTION };
    for i in 0..4 {
        let h = (6.0 + (i as f32) * 5.0) * s;
        let x = rect.left() + 2.0 * s + (i as f32) * 7.0 * s;
        let bar = Rect::from_min_size(Pos2::new(x, rect.bottom() - 3.0 * s - h), Vec2::new(4.5 * s, h));
        p.rect_filled(bar, 1.0 * s, if i < active { col } else { palette::SLATE_700 });
    }
}

/// Rotating vector compass dial.
pub fn compass_dial(ui: &mut Ui, size: f32, heading_deg: f32, target_deg: f32, color: Color32) {
    let s = size / 68.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let c = rect.center();
    let r = size * 0.44;
    let p = ui.painter();

    p.circle(c, r, palette::SCREEN_900, Stroke::new(1.4_f32, palette::SLATE_600));
    for k in 0..12 {
        let ang = (k as f32) * (std::f32::consts::TAU / 12.0);
        let d = Vec2::new(ang.sin(), -ang.cos());
        let len = if k % 3 == 0 { 5.0 * s } else { 2.5 * s };
        p.line_segment([c + d * (r - len), c + d * r], Stroke::new(1.1 * s, palette::TEXT_MUTED));
    }
    for (lbl, deg) in [("N", 0.0_f32), ("E", 90.0), ("S", 180.0), ("W", 270.0)] {
        let rad = (deg - heading_deg).to_radians();
        let pos = c + Vec2::new(rad.sin(), -rad.cos()) * (r - 10.0 * s);
        let col = if lbl == "N" { palette::CAUTION } else { palette::TEXT_MUTED };
        p.text(pos, Align2::CENTER_CENTER, lbl, FontId::monospace((10.5 * s).max(8.0)), col);
    }
    let rel = (target_deg - heading_deg).to_radians();
    let dir = Vec2::new(rel.sin(), -rel.cos());
    let tip = c + dir * (r * 0.72);
    let perp = Vec2::new(-dir.y, dir.x);
    p.line_segment([c - dir * (r * 0.25), tip], Stroke::new(2.0 * s, color));
    p.add(Shape::convex_polygon(
        vec![tip, tip - dir * (6.0 * s) + perp * (4.0 * s), tip - dir * (6.0 * s) - perp * (4.0 * s)],
        color,
        Stroke::NONE,
    ));
    p.circle_filled(c, 2.5 * s, palette::TEXT_WHITE);
}

/// 5-zone procedural vector spacesuit diagram (`[helmet, torso, scrubber, gloves, shield]`).
pub fn suit_schematic(ui: &mut Ui, height: f32, zone_colors: [Color32; 5]) {
    let s = height / 130.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(110.0 * s, height), Sense::hover());
    let p = ui.painter();
    let cx = rect.center().x;
    let top = rect.top() + 8.0 * s;

    p.circle_stroke(rect.center(), 56.0 * s, Stroke::new(1.2 * s, zone_colors[4].gamma_multiply(0.55)));
    let pack = Rect::from_min_size(Pos2::new(cx - 28.0 * s, top + 34.0 * s), Vec2::new(10.0 * s, 38.0 * s));
    p.rect(pack, 3.0 * s, zone_colors[2].gamma_multiply(0.24), Stroke::new(1.5 * s, zone_colors[2]));

    let head_c = Pos2::new(cx, top + 17.0 * s);
    p.circle(head_c, 15.0 * s, palette::SCREEN_900, Stroke::new(1.8 * s, zone_colors[0]));
    p.rect_filled(Rect::from_center_size(Pos2::new(cx + 2.0 * s, head_c.y), Vec2::new(16.0 * s, 9.5 * s)), 3.5 * s, zone_colors[0]);

    let torso = Rect::from_min_size(Pos2::new(cx - 16.0 * s, top + 35.0 * s), Vec2::new(32.0 * s, 40.0 * s));
    p.rect(torso, 6.0 * s, zone_colors[1].gamma_multiply(0.20), Stroke::new(1.8 * s, zone_colors[1]));
    p.circle_filled(torso.center(), 4.5 * s, zone_colors[1]);

    for sign in [-1.0_f32, 1.0_f32] {
        let shoulder = Pos2::new(if sign < 0.0 { torso.left() } else { torso.right() }, torso.top() + 5.0 * s);
        let hand = Pos2::new(shoulder.x + sign * 12.0 * s, torso.top() + 30.0 * s);
        p.line_segment([shoulder, hand], Stroke::new(4.2 * s, zone_colors[1]));
        p.circle_filled(hand, 4.5 * s, zone_colors[3]);
        p.line_segment(
            [Pos2::new(cx + sign * 8.0 * s, torso.bottom()), Pos2::new(cx + sign * 10.0 * s, torso.bottom() + 34.0 * s)],
            Stroke::new(5.5 * s, zone_colors[1]),
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(Frame::none().fill(palette::VOID_950).inner_margin(Margin::same(14.0)))
            .show(ctx, |ui| {
                ui.label(RichText::new("AEGIS-BIO // UI Component Showcase").font(heading_font(18.0)).color(palette::TEXT_WHITE));
                ui.add_space(10.0);

                ui.columns(4, |c| {
                    stat_card(&mut c[0], 96.0, "Heart rate", "88 bpm", palette::TEAL_ACCENT, "55–95 bpm", CardTheme::HEALTH);
                    stat_card(&mut c[1], 96.0, "Target distance", "222 m", palette::SKY_ACCENT, "Habitat", CardTheme::HUB);
                    stat_card(&mut c[2], 96.0, "Surface LAN", "Online", palette::NOMINAL, "192.168.1.10", CardTheme::COMMS);
                    stat_card(&mut c[3], 96.0, "Radiation", "0.42 mSv/d", palette::NOMINAL, "Limit < 2.0", CardTheme::HAZARD);
                });

                ui.add_space(12.0);
                ui.columns(2, |cols| {
                    cols[0].vertical(|ui| {
                        if let Some(v) = progress_slider(ui, 56.0, "Brightness", self.brightness, 25, 175, CardTheme::COMMS) {
                            self.brightness = v;
                        }
                        ui.add_space(8.0);
                        for (i, label) in ["Launch", "Orbit", "EVA", "Re-entry"].iter().enumerate() {
                            if touch_button(ui, ctx, ("btn", i), 44.0, label, if self.selected_idx == i { "Active" } else { "" }, self.selected_idx == i, CardTheme::HUB).clicked() {
                                self.selected_idx = i;
                            }
                            ui.add_space(6.0);
                        }
                    });

                    cols[1].vertical(|ui| {
                        Frame::none()
                            .fill(palette::SCREEN_900)
                            .stroke(Stroke::new(1.2_f32, palette::SLATE_700))
                            .rounding(Rounding::same(8.0))
                            .inner_margin(12.0)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    status_badge(ui, StatusLevel::Nominal, "Nominal");
                                    status_badge(ui, StatusLevel::Caution, "Caution");
                                    status_badge(ui, StatusLevel::Critical, "Critical");
                                    signal_bars(ui, 24.0, true);
                                });
                                ui.add_space(8.0);
                                corridor_bar(ui, ui.available_width(), 88.0, 55.0, 95.0, palette::TEAL_ACCENT);
                                ui.add_space(10.0);
                                ui.horizontal(|ui| {
                                    compass_dial(ui, 120.0, self.heading, 42.0, palette::SKY_ACCENT);
                                    ui.add_space(16.0);
                                    suit_schematic(ui, 120.0, [palette::NOMINAL, palette::NOMINAL, palette::CAUTION, palette::NOMINAL, palette::NOMINAL]);
                                });
                            });
                    });
                });
            });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([960.0, 540.0]).with_title("AEGIS-BIO // UI Components"),
        ..Default::default()
    };
    eframe::run_native(
        "AEGIS-BIO UI Components",
        options,
        Box::new(|cc| {
            install_aerospace_fonts(&cc.egui_ctx);
            let mut v = egui::Visuals::dark();
            v.panel_fill = palette::VOID_950;
            v.window_shadow = egui::epaint::Shadow::NONE;
            v.popup_shadow = egui::epaint::Shadow::NONE;
            cc.egui_ctx.set_visuals(v);
            Box::new(ShowcaseApp::default())
        }),
    )
}
