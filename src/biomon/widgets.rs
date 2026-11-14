use aegis_bio_demo::heading_font;
use eframe::egui::{
    self, Align2, Color32, CursorIcon, FontFamily, FontId, Id, Pos2, Rect, RichText, Sense, Shape,
    Stroke, Ui, Vec2,
};

use super::state::{Repairability, SuitZoneStatus};
use super::theme::{lerp_color, AccessibilityConfig, CardColors, StatusLevel, Tokens};

pub(crate) fn fit_16_by_9(bounds: Rect) -> Rect {
    let target = 16.0_f32 / 9.0_f32;
    let (w, h) = if bounds.width() / bounds.height() > target {
        (bounds.height() * target, bounds.height())
    } else {
        (bounds.width(), bounds.width() / target)
    };
    Rect::from_center_size(bounds.center(), Vec2::new(w, h))
}

pub(crate) fn zone_color(rep: Repairability, t: Tokens) -> Color32 {
    match rep {
        Repairability::Intact => t.nominal,
        Repairability::FieldRepairable => t.caution,
        Repairability::AirlockRequired => t.critical,
    }
}

pub(crate) fn draw_suit_schematic(
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

pub(crate) fn draw_working_compass(
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

    let ticks = if detailed { 12 } else { 8 };
    for k in 0..ticks {
        let ang = (k as f32) * (std::f32::consts::TAU / ticks as f32);
        let d = Vec2::new(ang.sin(), -ang.cos());
        let len = if k % (ticks / 4) == 0 { 5.0 * s } else { 2.5 * s };
        p.line_segment([c + d * (r - len), c + d * r], Stroke::new(1.1_f32 * s, t.text_muted));
    }

    let cards = [("N", 0.0_f32), ("E", 90.0), ("S", 180.0), ("W", 270.0)];
    let lbl_size = if detailed { 10.5 * s } else { 8.5 * s };
    for (lbl, deg) in cards {
        let rel_rad = (deg - heading_deg).to_radians();
        let pos = c + Vec2::new(rel_rad.sin(), -rel_rad.cos()) * (r - 10.0 * s);
        let col = if lbl == "N" { t.caution } else { t.text_muted };
        p.text(pos, Align2::CENTER_CENTER, lbl, FontId::monospace(lbl_size.max(8.0)), col);
    }

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

pub(crate) fn draw_status_shape(
    painter: &egui::Painter,
    center: Pos2,
    level: StatusLevel,
    color: Color32,
    t: Tokens,
) {
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

pub(crate) fn status_chip(ui: &mut Ui, level: StatusLevel, label: &str, color: Color32, t: Tokens) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::splat(t.sp(9.5).max(7.5)), Sense::hover());
        draw_status_shape(ui.painter(), r.center(), level, color, t);
        ui.label(RichText::new(label).font(heading_font(t.sz(12.5))).color(color));
    });
}

pub(crate) fn draw_signal_bars(
    ui: &mut Ui,
    size: f32,
    online: bool,
    on_col: Color32,
    off_col: Color32,
    t: Tokens,
) {
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

pub(crate) fn draw_corridor_bar(
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

pub(crate) fn gauge_pill(
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

pub(crate) fn stat_card_h(
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
pub(crate) fn brightness_slider(
    ui: &mut Ui,
    height: f32,
    value: u16,
    card_c: CardColors,
    t: Tokens,
) -> Option<u16> {
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

    let fill_rect = Rect::from_min_max(track.min, Pos2::new(track.left() + track.width() * frac, track.bottom()));
    p.rect_filled(track, r * 0.7, t.raised_bg);
    p.rect_filled(fill_rect, r * 0.7, card_c.accent.gamma_multiply(0.55));

    for pct in [50_u16, 100, 150] {
        let x = track.left() + track.width() * (pct - lo) as f32 / (hi - lo) as f32;
        let w = if pct == 100 { 2.0_f32 } else { 1.0_f32 };
        p.line_segment([Pos2::new(x, track.top()), Pos2::new(x, track.bottom())], Stroke::new(w, t.hairline_strong));
    }

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

pub(crate) fn color_button_h(
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

pub(crate) fn draw_heart_icon(ui: &mut Ui, size: f32, color: Color32, t: Tokens) {
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

pub(crate) fn draw_refresh_icon(painter: &egui::Painter, center: Pos2, r: f32, color: Color32, t: Tokens) {
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
