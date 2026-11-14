use eframe::egui::{self, Color32, Pos2, Stroke, Vec2, epaint::PathShape};
use std::f32::consts::TAU;

/// Procedural heart outline/fill that pulses +/- 15% on a sine wave synced to BPM.
pub fn draw_heart_icon(
    painter: &egui::Painter,
    center: Pos2,
    base_radius: f32,
    color: Color32,
    bpm: u16,
    time_sec: f64,
    filled: bool,
) {
    let freq_hz = (bpm.max(30) as f64) / 60.0;
    let pulse = (time_sec * freq_hz * std::f64::consts::TAU).sin() as f32;
    let scale = 1.0 + 0.15 * pulse;
    let r = base_radius * scale;

    // Parametric heart curve sampled into clean vector points
    let steps = 32;
    let mut points = Vec::with_capacity(steps);
    for i in 0..steps {
        let t = (i as f32 / steps as f32) * TAU;
        let x = 16.0 * t.sin().powi(3);
        let y = -(13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos());
        points.push(Pos2::new(
            center.x + (x / 17.0) * r,
            center.y + ((y - 1.5) / 17.0) * r,
        ));
    }

    let stroke = Stroke::new((base_radius * 0.14).clamp(2.0, 4.5), color);
    if filled {
        painter.add(egui::Shape::Path(PathShape::convex_polygon(
            points.clone(),
            color.gamma_multiply(0.16),
            Stroke::NONE,
        )));
    }
    painter.add(egui::Shape::Path(PathShape::closed_line(points, stroke)));
}

/// Procedural compass ring with directional heading arrow.
pub fn draw_compass_icon(
    painter: &egui::Painter,
    center: Pos2,
    radius: f32,
    color: Color32,
    heading_deg: f32,
) {
    let stroke_w = (radius * 0.13).clamp(2.0, 4.5);
    painter.circle_stroke(center, radius, Stroke::new(stroke_w, color));

    // Cardinal tick marks
    for angle_deg in [0.0_f32, 90.0, 180.0, 270.0] {
        let rad = angle_deg.to_radians();
        let dir = Vec2::new(rad.sin(), -rad.cos());
        painter.line_segment(
            [center + dir * (radius * 0.76), center + dir * radius],
            Stroke::new(stroke_w * 0.85, color),
        );
    }

    // Rotating heading arrow (0 deg = North / Up)
    let rad = heading_deg.to_radians();
    let fwd = Vec2::new(rad.sin(), -rad.cos());
    let right = Vec2::new(fwd.y, -fwd.x);

    let tip = center + fwd * (radius * 0.72);
    let left_wing = center - fwd * (radius * 0.25) - right * (radius * 0.34);
    let notch = center - fwd * (radius * 0.08);
    let right_wing = center - fwd * (radius * 0.25) + right * (radius * 0.34);

    painter.add(egui::Shape::Path(PathShape::convex_polygon(
        vec![tip, right_wing, notch, left_wing],
        color,
        Stroke::new(1.5, color),
    )));
}

/// Procedural suit / pressure shield outline.
pub fn draw_shield_icon(painter: &egui::Painter, center: Pos2, radius: f32, color: Color32) {
    let stroke_w = (radius * 0.14).clamp(2.0, 4.5);
    let points = vec![
        Pos2::new(center.x, center.y - radius * 0.92),
        Pos2::new(center.x + radius * 0.82, center.y - radius * 0.56),
        Pos2::new(center.x + radius * 0.72, center.y + radius * 0.22),
        Pos2::new(center.x, center.y + radius * 0.96),
        Pos2::new(center.x - radius * 0.72, center.y + radius * 0.22),
        Pos2::new(center.x - radius * 0.82, center.y - radius * 0.56),
    ];
    painter.add(egui::Shape::Path(PathShape::closed_line(
        points,
        Stroke::new(stroke_w, color),
    )));

    // Inner cross / visor bar
    painter.line_segment(
        [
            Pos2::new(center.x - radius * 0.38, center.y - radius * 0.12),
            Pos2::new(center.x + radius * 0.38, center.y - radius * 0.12),
        ],
        Stroke::new(stroke_w * 0.85, color),
    );
    painter.line_segment(
        [
            Pos2::new(center.x, center.y - radius * 0.45),
            Pos2::new(center.x, center.y + radius * 0.42),
        ],
        Stroke::new(stroke_w * 0.85, color),
    );
}

/// Procedural hazard / radiation warning triangle.
pub fn draw_hazard_icon(painter: &egui::Painter, center: Pos2, radius: f32, color: Color32) {
    let stroke_w = (radius * 0.14).clamp(2.0, 4.5);
    let top = Pos2::new(center.x, center.y - radius * 0.88);
    let br = Pos2::new(center.x + radius * 0.92, center.y + radius * 0.74);
    let bl = Pos2::new(center.x - radius * 0.92, center.y + radius * 0.74);

    painter.add(egui::Shape::Path(PathShape::closed_line(
        vec![top, br, bl],
        Stroke::new(stroke_w, color),
    )));

    // Exclamation mark inside warning triangle
    painter.line_segment(
        [
            Pos2::new(center.x, center.y - radius * 0.30),
            Pos2::new(center.x, center.y + radius * 0.25),
        ],
        Stroke::new(stroke_w * 1.1, color),
    );
    painter.circle_filled(
        Pos2::new(center.x, center.y + radius * 0.48),
        (stroke_w * 0.65).max(2.0),
        color,
    );
}

/// Procedural hydration / water droplet icon.
pub fn draw_droplet_icon(painter: &egui::Painter, center: Pos2, radius: f32, color: Color32) {
    let stroke_w = (radius * 0.14).clamp(2.0, 4.0);
    let mut points = Vec::new();
    let tip = Pos2::new(center.x, center.y - radius * 0.95);
    points.push(tip);

    let arc_steps = 16;
    for i in 0..=arc_steps {
        let angle = -0.15 * std::f32::consts::PI
            + (i as f32 / arc_steps as f32) * (1.30 * std::f32::consts::PI);
        let cx = center.x + angle.cos() * (radius * 0.66);
        let cy = center.y + radius * 0.22 + angle.sin() * (radius * 0.68);
        points.push(Pos2::new(cx, cy));
    }

    painter.add(egui::Shape::Path(PathShape::closed_line(
        points,
        Stroke::new(stroke_w, color),
    )));
}

/// Procedural stepped signal bars + radio wave arc for Comms.
pub fn draw_comms_icon(
    painter: &egui::Painter,
    center: Pos2,
    radius: f32,
    color: Color32,
    online: bool,
) {
    let stroke_w = (radius * 0.13).clamp(2.0, 4.0);
    let bar_count = 4;
    let total_w = radius * 1.6;
    let step_w = total_w / bar_count as f32;
    let bar_w = step_w * 0.62;
    let base_y = center.y + radius * 0.72;
    let start_x = center.x - total_w * 0.5;

    for i in 0..bar_count {
        let frac = (i + 1) as f32 / bar_count as f32;
        let h = radius * 1.35 * frac;
        let x = start_x + i as f32 * step_w;
        let rect = egui::Rect::from_min_max(Pos2::new(x, base_y - h), Pos2::new(x + bar_w, base_y));
        if online || i == 0 {
            painter.rect_filled(rect, 2.0, color);
        } else {
            painter.rect_stroke(
                rect,
                2.0,
                Stroke::new(stroke_w * 0.8, color),
                egui::StrokeKind::Inside,
            );
        }
    }

    if !online {
        painter.line_segment(
            [
                Pos2::new(center.x - radius * 0.85, center.y - radius * 0.75),
                Pos2::new(center.x + radius * 0.85, center.y + radius * 0.75),
            ],
            Stroke::new(stroke_w * 1.15, color),
        );
    }
}
