use crate::app::{AppMode, BiomonApp, QuadrantId, Screen};
use crate::calc;
use crate::icons;
use crate::net::ConnectionState;
use crate::ui_components::{self, CardTheme, palette};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, Sense, Stroke, StrokeKind,
    Vec2, epaint::Shadow,
};

// Re-export mandated palette constants
pub use palette::{
    BG_ACTIVE, BG_HOVER, BG_PRIMARY, INK_PRIMARY, OUTLINE_ALERT, OUTLINE_INPUT, OUTLINE_VIEW,
};

const BORDER_NOMINAL: f32 = 5.0;
const BORDER_ALERT: f32 = 7.0;

pub fn apply_mandated_visuals(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();
    visuals.dark_mode = false;
    visuals.panel_fill = BG_PRIMARY;
    visuals.window_fill = BG_PRIMARY;
    visuals.extreme_bg_color = BG_PRIMARY;
    visuals.faint_bg_color = Color32::from_rgb(232, 232, 218);
    visuals.override_text_color = Some(INK_PRIMARY);
    visuals.window_shadow = Shadow::NONE;
    visuals.popup_shadow = Shadow::NONE;
    visuals.window_stroke = Stroke::new(BORDER_NOMINAL, INK_PRIMARY);

    visuals.widgets.noninteractive.bg_fill = BG_PRIMARY;
    visuals.widgets.noninteractive.weak_bg_fill = BG_PRIMARY;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(BORDER_NOMINAL, OUTLINE_VIEW);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(2.0, INK_PRIMARY);

    visuals.widgets.inactive.bg_fill = BG_PRIMARY;
    visuals.widgets.inactive.weak_bg_fill = BG_PRIMARY;
    visuals.widgets.inactive.bg_stroke = Stroke::new(BORDER_NOMINAL, OUTLINE_VIEW);
    visuals.widgets.inactive.fg_stroke = Stroke::new(2.0, INK_PRIMARY);

    visuals.widgets.hovered.bg_fill = BG_HOVER;
    visuals.widgets.hovered.weak_bg_fill = BG_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(BORDER_NOMINAL, OUTLINE_INPUT);
    visuals.widgets.hovered.fg_stroke = Stroke::new(2.5, INK_PRIMARY);

    visuals.widgets.active.bg_fill = BG_ACTIVE;
    visuals.widgets.active.weak_bg_fill = BG_ACTIVE;
    visuals.widgets.active.bg_stroke = Stroke::new(BORDER_NOMINAL, OUTLINE_INPUT);
    visuals.widgets.active.fg_stroke = Stroke::new(2.5, INK_PRIMARY);

    ctx.set_visuals(visuals);
}

pub fn render(ui: &mut egui::Ui, app: &mut BiomonApp) {
    ui.ctx().request_repaint();

    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(BG_PRIMARY))
        .show(ui, |ui| match app.screen {
            Screen::MainHud => render_main_hud(ui, app),
            Screen::Detail(quadrant) => render_detail_screen(ui, app, quadrant),
        });
}

fn mode_outline_color(mode: AppMode) -> Color32 {
    match mode {
        AppMode::ViewMode => OUTLINE_VIEW,
        AppMode::InputMode => OUTLINE_INPUT,
    }
}

fn quadrant_border(mode: AppMode, is_alert: bool, time_sec: f64) -> (Color32, f32) {
    if is_alert {
        let pulse = ((time_sec * 4.5).sin() * 0.5 + 0.5) as f32;
        let width = BORDER_NOMINAL + (BORDER_ALERT - BORDER_NOMINAL) * pulse;
        (OUTLINE_ALERT, width.max(BORDER_ALERT - 0.5))
    } else {
        (mode_outline_color(mode), BORDER_NOMINAL)
    }
}

fn render_main_hud(ui: &mut egui::Ui, app: &mut BiomonApp) {
    let screen_rect = ui.max_rect();
    let (_, response) = ui.allocate_exact_size(screen_rect.size(), Sense::click());
    let painter = ui.painter_at(screen_rect);
    let time_sec = ui.input(|i| i.time);

    painter.rect_filled(screen_rect, CornerRadius::ZERO, BG_PRIMARY);

    let min_dim = screen_rect.width().min(screen_rect.height());
    let center = screen_rect.center();
    let circle_radius = (min_dim * 0.165).clamp(38.0, 140.0);

    let outer_pad = (min_dim * 0.01).clamp(3.0, 8.0);
    let gap = (min_dim * 0.012).clamp(4.0, 10.0);
    let inner_rect = screen_rect.shrink(outer_pad);
    let mid_x = inner_rect.center().x;
    let mid_y = inner_rect.center().y;
    let half_gap = gap * 0.5;

    let tl_rect = Rect::from_min_max(
        inner_rect.min,
        Pos2::new(mid_x - half_gap, mid_y - half_gap),
    );
    let tr_rect = Rect::from_min_max(
        Pos2::new(mid_x + half_gap, inner_rect.min.y),
        Pos2::new(inner_rect.max.x, mid_y - half_gap),
    );
    let bl_rect = Rect::from_min_max(
        Pos2::new(inner_rect.min.x, mid_y + half_gap),
        Pos2::new(mid_x - half_gap, inner_rect.max.y),
    );
    let br_rect = Rect::from_min_max(
        Pos2::new(mid_x + half_gap, mid_y + half_gap),
        inner_rect.max,
    );

    let hover_pos = response.hover_pos();
    let hover_in_circle = hover_pos.is_some_and(|p| p.distance(center) <= circle_radius);
    let pointer_down = ui.input(|i| i.pointer.primary_down());

    // CRITICAL HIT-TESTING RULE: Check pointer distance from center FIRST.
    if response.clicked()
        && let Some(click_pos) = response.interact_pointer_pos()
    {
        if click_pos.distance(center) <= circle_radius {
            app.mode.toggle();
        } else if tl_rect.contains(click_pos) {
            app.screen = Screen::Detail(QuadrantId::HealthAndSuit);
        } else if tr_rect.contains(click_pos) {
            app.screen = Screen::Detail(QuadrantId::HubAndNav);
        } else if bl_rect.contains(click_pos) {
            app.screen = Screen::Detail(QuadrantId::CommsAndDisplay);
        } else if br_rect.contains(click_pos) {
            app.screen = Screen::Detail(QuadrantId::HazardAndEnv);
        }
    }

    let snap = &app.snapshot;
    let q_h = tl_rect.height();
    let q_w = tl_rect.width();
    let pad_x = (q_w * 0.048).clamp(8.0, 24.0);
    let pad_y = (q_h * 0.055).clamp(6.0, 18.0);
    let title_sz = (q_h * 0.11).min(q_w * 0.082).clamp(11.0, 32.0);
    let hero_sz = (q_h * 0.21).min(q_w * 0.145).clamp(15.0, 56.0);
    let sub_sz = (q_h * 0.088).min(q_w * 0.058).clamp(9.0, 26.0);
    let icon_r = (title_sz * 0.56).clamp(7.0, 20.0);

    // -------------------------------------------------------------------------
    // 1. TOP-LEFT: HEALTH & SUIT
    // -------------------------------------------------------------------------
    let health_alert = snap.alerts.health_danger;
    let (tl_border, tl_stroke_w) = quadrant_border(app.mode, health_alert, time_sec);
    let tl_hovered = !hover_in_circle && hover_pos.is_some_and(|p| tl_rect.contains(p));
    let tl_fill = panel_fill(tl_hovered, tl_hovered && pointer_down);
    draw_panel_box(&painter, tl_rect, tl_fill, tl_border, tl_stroke_w);

    let health_ink = if health_alert {
        OUTLINE_ALERT
    } else {
        INK_PRIMARY
    };

    painter.text(
        Pos2::new(tl_rect.min.x + pad_x, tl_rect.min.y + pad_y),
        Align2::LEFT_TOP,
        "HEALTH & SUIT",
        FontId::new(title_sz, FontFamily::Proportional),
        INK_PRIMARY,
    );

    let heart_y = tl_rect.min.y + pad_y + title_sz + (q_h * 0.13).clamp(12.0, 48.0);
    let heart_r = (hero_sz * 0.36).clamp(7.0, 22.0);
    let heart_center = Pos2::new(tl_rect.min.x + pad_x + heart_r, heart_y);
    let bpm_u16 = snap.hr_bpm.round().clamp(30.0, 240.0) as u16;
    icons::draw_heart_icon(
        &painter,
        heart_center,
        heart_r,
        health_ink,
        bpm_u16,
        time_sec,
        true,
    );
    painter.text(
        Pos2::new(heart_center.x + heart_r * 1.65, heart_center.y),
        Align2::LEFT_CENTER,
        format!("{:.0} BPM", snap.hr_bpm),
        FontId::new(hero_sz, FontFamily::Monospace),
        health_ink,
    );

    let suit_p = snap.suit_pressure_kpa.unwrap_or(29.6);
    let suit_ok = suit_p >= 25.0;
    let row2_y = heart_y + hero_sz * 0.62 + (q_h * 0.04).clamp(4.0, 16.0);
    painter.text(
        Pos2::new(tl_rect.min.x + pad_x, row2_y),
        Align2::LEFT_TOP,
        format!(
            "SBP {:.0} | SPO2 {:.0}% | {:.1}KPA",
            snap.sbp_mmhg, snap.spo2_pct, suit_p
        ),
        FontId::new(sub_sz, FontFamily::Monospace),
        INK_PRIMARY,
    );

    let row3_y = row2_y + sub_sz * 1.45;
    let drop_r = (sub_sz * 0.48).clamp(5.0, 14.0);
    let droplet_c = Pos2::new(tl_rect.min.x + pad_x + drop_r, row3_y + sub_sz * 0.52);
    icons::draw_droplet_icon(&painter, droplet_c, drop_r, INK_PRIMARY);
    painter.text(
        Pos2::new(droplet_c.x + drop_r * 1.5, row3_y),
        Align2::LEFT_TOP,
        format!(
            "{:.0} ML/H | SI {:.2} {}",
            snap.sweat_ml_per_sec * 3600.0,
            snap.shock_index,
            snap.orthostatic_risk
        ),
        FontId::new(sub_sz * 0.96, FontFamily::Monospace),
        INK_PRIMARY,
    );

    // Safe HR corridor bar near bottom-left of TL quadrant (stopping left of center circle)
    let (hr_lo, hr_hi) = calc::phase_hr_corridor(&snap.active_phase);
    let corridor_w = (q_w - circle_radius - pad_x * 1.6).clamp(56.0, 360.0);
    let corridor_h = (q_h * 0.068).clamp(10.0, 22.0);
    let corridor_rect = Rect::from_min_size(
        Pos2::new(tl_rect.min.x + pad_x, tl_rect.max.y - pad_y - corridor_h),
        Vec2::new(corridor_w, corridor_h),
    );
    ui_components::paint_corridor_bar(
        &painter,
        corridor_rect,
        snap.hr_bpm as f32,
        hr_lo as f32,
        hr_hi as f32,
        if health_alert {
            OUTLINE_ALERT
        } else {
            OUTLINE_VIEW
        },
    );

    // 5-zone spacesuit schematic in top-right of Top-Left quadrant
    let suit_zone_col = if suit_ok { OUTLINE_VIEW } else { OUTLINE_ALERT };
    let suit_h = (q_h * 0.34).min(q_w * 0.24).clamp(38.0, 124.0);
    let suit_w = suit_h * (110.0 / 130.0);
    let suit_rect = Rect::from_min_size(
        Pos2::new(tl_rect.max.x - suit_w - pad_x * 0.75, tl_rect.min.y + pad_y * 0.8),
        Vec2::new(suit_w, suit_h),
    );
    ui_components::paint_suit_schematic(
        &painter,
        suit_rect,
        [
            INK_PRIMARY,
            suit_zone_col,
            if snap.cabin_co2_ppm > 3000.0 {
                OUTLINE_ALERT
            } else {
                OUTLINE_VIEW
            },
            INK_PRIMARY,
            suit_zone_col,
        ],
    );

    // -------------------------------------------------------------------------
    // 2. TOP-RIGHT: HUB & NAV
    // -------------------------------------------------------------------------
    let hub_alert = snap.alerts.hub_danger;
    let (tr_border, tr_stroke_w) = quadrant_border(app.mode, hub_alert, time_sec);
    let tr_hovered = !hover_in_circle && hover_pos.is_some_and(|p| tr_rect.contains(p));
    let tr_fill = panel_fill(tr_hovered, tr_hovered && pointer_down);
    draw_panel_box(&painter, tr_rect, tr_fill, tr_border, tr_stroke_w);

    let tr_right = tr_rect.max.x - pad_x;
    let tr_top = tr_rect.min.y + pad_y;
    let hub_ink = if hub_alert {
        OUTLINE_ALERT
    } else {
        INK_PRIMARY
    };

    // Compass dial in upper-left of Top-Right quadrant
    let compass_sz = (q_h * 0.34).min(q_w * 0.24).clamp(38.0, 120.0);
    let compass_rect = Rect::from_min_size(
        Pos2::new(tr_rect.min.x + pad_x * 0.8, tr_rect.min.y + pad_y * 0.8),
        Vec2::splat(compass_sz),
    );
    ui_components::paint_compass_dial(
        &painter,
        compass_rect,
        snap.heading_deg,
        snap.target_bearing_deg,
        if hub_alert {
            OUTLINE_ALERT
        } else {
            OUTLINE_INPUT
        },
    );

    painter.text(
        Pos2::new(tr_right, tr_top),
        Align2::RIGHT_TOP,
        "HUB & NAV",
        FontId::new(title_sz, FontFamily::Proportional),
        INK_PRIMARY,
    );
    painter.text(
        Pos2::new(tr_right, tr_top + title_sz + (q_h * 0.04).clamp(4.0, 18.0)),
        Align2::RIGHT_TOP,
        format!("{} M", snap.hub_distance_m),
        FontId::new(hero_sz, FontFamily::Monospace),
        hub_ink,
    );
    painter.text(
        Pos2::new(
            tr_right,
            tr_top + title_sz + hero_sz + (q_h * 0.07).clamp(8.0, 28.0),
        ),
        Align2::RIGHT_TOP,
        format!(
            "{:03.0}° {} | {}",
            snap.heading_deg,
            snap.cardinal_heading(),
            snap.active_phase
        ),
        FontId::new(sub_sz, FontFamily::Monospace),
        INK_PRIMARY,
    );
    painter.text(
        Pos2::new(tr_right, tr_rect.max.y - pad_y),
        Align2::RIGHT_BOTTOM,
        format!(
            "{} | SOL {:02} | {:.1}G",
            snap.callsign, snap.mission_day, snap.g_load
        ),
        FontId::new(sub_sz, FontFamily::Monospace),
        INK_PRIMARY,
    );

    // -------------------------------------------------------------------------
    // 3. BOTTOM-LEFT: COMMS
    // -------------------------------------------------------------------------
    let comms_alert = snap.alerts.comms_danger;
    let (bl_border, bl_stroke_w) = quadrant_border(app.mode, comms_alert, time_sec);
    let bl_hovered = !hover_in_circle && hover_pos.is_some_and(|p| bl_rect.contains(p));
    let bl_fill = panel_fill(bl_hovered, bl_hovered && pointer_down);
    draw_panel_box(&painter, bl_rect, bl_fill, bl_border, bl_stroke_w);

    let bl_left = bl_rect.min.x + pad_x;
    let bl_top = bl_rect.min.y + pad_y;
    let bl_bottom = bl_rect.max.y - pad_y;
    let comms_color = if comms_alert {
        OUTLINE_ALERT
    } else {
        INK_PRIMARY
    };

    painter.text(
        Pos2::new(bl_left, bl_top),
        Align2::LEFT_TOP,
        "COMMS",
        FontId::new(title_sz, FontFamily::Proportional),
        INK_PRIMARY,
    );

    painter.text(
        Pos2::new(bl_left, bl_top + title_sz + (q_h * 0.05).clamp(6.0, 18.0)),
        Align2::LEFT_TOP,
        format!(
            "DELAY {:04.1}M | {:.2} AU",
            snap.one_way_light_min, snap.earth_mars_au
        ),
        FontId::new(sub_sz, FontFamily::Monospace),
        INK_PRIMARY,
    );

    painter.text(
        Pos2::new(
            bl_left,
            bl_top + title_sz + sub_sz * 1.45 + (q_h * 0.06).clamp(6.0, 22.0),
        ),
        Align2::LEFT_TOP,
        format!(
            "QUEUE {} | TX {}",
            snap.unsynced.total(),
            app.lan_client.frames_sent()
        ),
        FontId::new(sub_sz, FontFamily::Monospace),
        INK_PRIMARY,
    );

    let sig_sz = (q_h * 0.16).min(q_w * 0.14).clamp(20.0, 50.0);
    let sig_rect = Rect::from_min_size(Pos2::new(bl_left, bl_bottom - sig_sz), Vec2::splat(sig_sz));
    ui_components::paint_signal_bars(
        &painter,
        sig_rect,
        snap.comms_state == ConnectionState::Online,
        comms_alert,
    );

    painter.text(
        Pos2::new(bl_left + sig_sz + (pad_x * 0.6).clamp(6.0, 14.0), bl_bottom),
        Align2::LEFT_BOTTOM,
        snap.comms_state.label(),
        FontId::new(hero_sz * 0.94, FontFamily::Monospace),
        comms_color,
    );

    // -------------------------------------------------------------------------
    // 4. BOTTOM-RIGHT: HAZARD & ENV
    // -------------------------------------------------------------------------
    let hazard_alert = snap.alerts.hazard_danger;
    let (br_border, br_stroke_w) = quadrant_border(app.mode, hazard_alert, time_sec);
    let br_hovered = !hover_in_circle && hover_pos.is_some_and(|p| br_rect.contains(p));
    let br_fill = panel_fill(br_hovered, br_hovered && pointer_down);
    draw_panel_box(&painter, br_rect, br_fill, br_border, br_stroke_w);

    let br_right = br_rect.max.x - pad_x;
    let br_top = br_rect.min.y + pad_y;
    let br_bottom = br_rect.max.y - pad_y;
    let hazard_color = if hazard_alert {
        OUTLINE_ALERT
    } else {
        INK_PRIMARY
    };
    let hazard_status = if hazard_alert { "ALERT" } else { "SAFE" };

    icons::draw_hazard_icon(
        &painter,
        Pos2::new(br_right - icon_r, br_top + icon_r),
        icon_r,
        hazard_color,
    );
    painter.text(
        Pos2::new(br_right - icon_r * 2.4, br_top),
        Align2::RIGHT_TOP,
        "HAZARD & ENV",
        FontId::new(title_sz, FontFamily::Proportional),
        INK_PRIMARY,
    );

    let sans_label = snap
        .sans_state
        .as_ref()
        .map(|s| s.sans_risk.as_str())
        .unwrap_or("LOW");
    painter.text(
        Pos2::new(br_right, br_top + title_sz + (q_h * 0.05).clamp(6.0, 18.0)),
        Align2::RIGHT_TOP,
        format!("CO2 {:.0} PPM | SANS {}", snap.cabin_co2_ppm, sans_label),
        FontId::new(sub_sz, FontFamily::Monospace),
        INK_PRIMARY,
    );

    painter.text(
        Pos2::new(br_right, br_bottom - sub_sz * 1.45),
        Align2::RIGHT_BOTTOM,
        hazard_status,
        FontId::new(hero_sz * 0.94, FontFamily::Proportional),
        hazard_color,
    );

    painter.text(
        Pos2::new(br_right, br_bottom),
        Align2::RIGHT_BOTTOM,
        format!(
            "O2 {:.1}% | {:.2} USV/H",
            snap.cabin_o2_pct.unwrap_or(20.9),
            snap.radiation_usv_hr
        ),
        FontId::new(sub_sz, FontFamily::Monospace),
        hazard_color,
    );

    // -------------------------------------------------------------------------
    // CENTER CIRCLE BUTTON (CYCLE)
    // -------------------------------------------------------------------------
    let circle_border = mode_outline_color(app.mode);
    let circle_fill = panel_fill(hover_in_circle, hover_in_circle && pointer_down);

    painter.circle(
        center,
        circle_radius,
        circle_fill,
        Stroke::new(BORDER_NOMINAL, circle_border),
    );

    let cycle_lbl_sz = (circle_radius * 0.24).clamp(10.0, 34.0);
    let cycle_mode_sz = (circle_radius * 0.29).clamp(12.0, 40.0);
    painter.text(
        Pos2::new(center.x, center.y - circle_radius * 0.18),
        Align2::CENTER_CENTER,
        "CYCLE",
        FontId::new(cycle_lbl_sz, FontFamily::Proportional),
        INK_PRIMARY,
    );
    painter.text(
        Pos2::new(center.x, center.y + circle_radius * 0.18),
        Align2::CENTER_CENTER,
        app.mode.cycle_badge_label(),
        FontId::new(cycle_mode_sz, FontFamily::Proportional),
        INK_PRIMARY,
    );
}

fn panel_fill(hovered: bool, pressed: bool) -> Color32 {
    if pressed {
        BG_ACTIVE
    } else if hovered {
        BG_HOVER
    } else {
        BG_PRIMARY
    }
}

fn draw_panel_box(
    painter: &egui::Painter,
    rect: Rect,
    fill: Color32,
    border_color: Color32,
    stroke_width: f32,
) {
    painter.rect(
        rect,
        CornerRadius::same(10),
        fill,
        Stroke::new(stroke_width, border_color),
        StrokeKind::Inside,
    );
}

fn render_detail_screen(ui: &mut egui::Ui, app: &mut BiomonApp, quadrant: QuadrantId) {
    let screen_rect = ui.max_rect();
    let time_sec = ui.input(|i| i.time);
    let min_dim = screen_rect.width().min(screen_rect.height());
    let is_alert = match quadrant {
        QuadrantId::HealthAndSuit => app.snapshot.alerts.health_danger,
        QuadrantId::HubAndNav => app.snapshot.alerts.hub_danger,
        QuadrantId::CommsAndDisplay => app.snapshot.alerts.comms_danger,
        QuadrantId::HazardAndEnv => app.snapshot.alerts.hazard_danger,
    };

    let (border_color, border_width) = quadrant_border(app.mode, is_alert, time_sec);
    let outer_pad = (min_dim * 0.01).clamp(3.0, 8.0);
    let outer_rect = screen_rect.shrink(outer_pad);
    let theme = CardTheme::for_mobile(app.mode == AppMode::InputMode, is_alert);

    let painter = ui.painter();
    painter.rect(
        outer_rect,
        CornerRadius::same(10),
        BG_PRIMARY,
        Stroke::new(border_width, border_color),
        StrokeKind::Inside,
    );

    let pad_x = (outer_rect.width() * 0.022).clamp(8.0, 20.0);
    let pad_y = (outer_rect.height() * 0.022).clamp(6.0, 14.0);
    let inner_rect = outer_rect.shrink2(Vec2::new(pad_x, pad_y));

    if app.mode == AppMode::ViewMode {
        // VIEW MODE: No title, no header RETURN button — entire screen is tap-anywhere RETURN.
        let tap_return = ui.interact(
            outer_rect,
            ui.scope_id().with(("view_tap_return", quadrant as u8)),
            Sense::click(),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(inner_rect), |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            match quadrant {
                QuadrantId::HealthAndSuit => render_health_detail(ui, app, theme, is_alert),
                QuadrantId::HubAndNav => render_hub_nav_detail(ui, app, theme, is_alert),
                QuadrantId::CommsAndDisplay => render_comms_detail(ui, app, theme, is_alert),
                QuadrantId::HazardAndEnv => render_hazard_detail(ui, app, theme, is_alert),
            }
        });
        if tap_return.clicked() {
            app.screen = Screen::MainHud;
        }
    } else {
        // INPUT MODE: No top title clutter; interactive controls above a large bottom RETURN button.
        let gap = (inner_rect.height() * 0.018).clamp(6.0, 12.0);
        let ret_h = (inner_rect.height() * 0.13).clamp(50.0, 96.0);
        let content_rect = Rect::from_min_max(
            inner_rect.min,
            Pos2::new(inner_rect.max.x, (inner_rect.max.y - ret_h - gap).max(inner_rect.min.y + 40.0)),
        );
        let ret_rect = Rect::from_min_max(
            Pos2::new(inner_rect.min.x, content_rect.max.y + gap),
            inner_rect.max,
        );

        ui.scope_builder(egui::UiBuilder::new().max_rect(content_rect), |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| match quadrant {
                    QuadrantId::HealthAndSuit => render_health_detail(ui, app, theme, is_alert),
                    QuadrantId::HubAndNav => render_hub_nav_detail(ui, app, theme, is_alert),
                    QuadrantId::CommsAndDisplay => render_comms_detail(ui, app, theme, is_alert),
                    QuadrantId::HazardAndEnv => render_hazard_detail(ui, app, theme, is_alert),
                });
        });

        ui.scope_builder(egui::UiBuilder::new().max_rect(ret_rect), |ui| {
            let ctx = ui.ctx().clone();
            let w = ui.available_width();
            if ui_components::touch_button(
                ui,
                &ctx,
                ("input_return_btn", quadrant as u8),
                Vec2::new(w, ret_rect.height()),
                "RETURN",
                "",
                false,
                theme,
            )
            .clicked()
            {
                app.screen = Screen::MainHud;
            }
        });
    }
}

fn render_health_detail(ui: &mut egui::Ui, app: &mut BiomonApp, theme: CardTheme, is_alert: bool) {
    let ctx = ui.ctx().clone();
    if app.mode == AppMode::ViewMode {
        let snap = &app.snapshot;
        let val_col = if is_alert { OUTLINE_ALERT } else { INK_PRIMARY };
        let avail_w = ui.available_width();
        let avail_h = ui.available_height();
        let narrow = avail_w < 540.0;
        let gap = (avail_h * 0.02).clamp(4.0, 10.0);

        let cards: [(&str, String, Color32); 8] = [
            ("HEART RATE", format!("{:.0} BPM", snap.hr_bpm), val_col),
            (
                "CUFF SBP / RESP",
                format!("{:.0} / {:.0}", snap.sbp_mmhg, snap.resp_brpm),
                INK_PRIMARY,
            ),
            (
                "SPO2 / TISSUE O2",
                format!("{:.0}% / {:.0}%", snap.spo2_pct, snap.tissue_o2_pct.unwrap_or(93.5)),
                val_col,
            ),
            (
                "N2 R-IDX / SHOCK",
                format!("{:.2} / {:.2}", snap.blood_n2_idx.unwrap_or(1.08), snap.shock_index),
                val_col,
            ),
            (
                "SUIT P / HUMID",
                format!(
                    "{:.1} KPA / {:.0}%",
                    snap.suit_pressure_kpa.unwrap_or(29.6),
                    snap.suit_humidity_pct.unwrap_or(38.0)
                ),
                INK_PRIMARY,
            ),
            (
                "SKIN / BURN RATE",
                format!("{:.1}C / {:.0} KCAL/H", snap.skin_temp_c, snap.kcal_per_sec * 3600.0),
                INK_PRIMARY,
            ),
            (
                "SWEAT / FLUID BAL",
                format!(
                    "{:.0} ML / {:.0} ML",
                    snap.daily_physio.sweat_loss_ml_total,
                    snap.daily_nutrition.fluid_ml_total
                ),
                if (snap.daily_physio.sweat_loss_ml_total - snap.daily_nutrition.fluid_ml_total)
                    > 1500.0
                {
                    OUTLINE_ALERT
                } else {
                    INK_PRIMARY
                },
            ),
            (
                "OSTEO / ORTHO",
                format!(
                    "{} / {}",
                    snap.osteo_state.as_ref().map(|o| o.osteo_risk.as_str()).unwrap_or("LOW"),
                    snap.orthostatic_risk
                ),
                INK_PRIMARY,
            ),
        ];

        let cols_per_row = if narrow { 2 } else { 4 };
        let card_rows = 8 / cols_per_row;
        let top_frac = if narrow { 0.62 } else { 0.56 };
        let row_h = ((avail_h * top_frac - gap * (card_rows as f32)) / card_rows as f32).max(40.0);

        for row_idx in 0..card_rows {
            let start = row_idx * cols_per_row;
            ui.columns(cols_per_row, |cols| {
                for c in 0..cols_per_row {
                    let (lbl, val, col) = &cards[start + c];
                    ui_components::stat_card(&mut cols[c], row_h, lbl, val, *col, theme);
                }
            });
            ui.add_space(gap);
        }

        let rem_h = ui.available_height().max(56.0);
        let (lo, hi) = calc::phase_hr_corridor(&snap.active_phase);
        let hdr_text = format!(
            "PHASE {} HR ({:.0}–{:.0}) | EVA {:02}:{:02}",
            snap.active_phase,
            lo,
            hi,
            snap.eva_elapsed_sec / 60,
            snap.eva_elapsed_sec % 60
        );

        ui.allocate_ui(Vec2::new(avail_w, rem_h), |ui| {
            ui.columns(2, |cols| {
                let col0_w = cols[0].available_width();
                let hdr_chars = hdr_text.chars().count().max(1) as f32;
                let hdr_sz = (rem_h * 0.12)
                    .min(col0_w / (hdr_chars * 0.56))
                    .clamp(9.0, 24.0);
                cols[0].add(
                    egui::Label::new(
                        egui::RichText::new(&hdr_text)
                            .size(hdr_sz)
                            .strong()
                            .color(INK_PRIMARY),
                    )
                    .truncate(),
                );
                cols[0].add_space(gap * 0.45);
                ui_components::corridor_bar(
                    &mut cols[0],
                    col0_w,
                    snap.hr_bpm as f32,
                    lo as f32,
                    hi as f32,
                    if is_alert { OUTLINE_ALERT } else { OUTLINE_VIEW },
                );
                cols[0].add_space(gap * 0.55);
                let nut_h = cols[0].available_height().max(38.0);
                ui_components::stat_card(
                    &mut cols[0],
                    nut_h,
                    "SOL NUTRITION (IN / BURN / NA / CA)",
                    &format!(
                        "{:.0}/{:.0} KCAL | {:.0} NA | {:.0} CA",
                        snap.daily_nutrition.kcal_total,
                        snap.daily_physio.kcal_burned_total,
                        snap.daily_nutrition.sodium_mg_total,
                        snap.daily_nutrition.calcium_mg_total
                    ),
                    INK_PRIMARY,
                    theme,
                );

                cols[1].horizontal(|ui| {
                    let suit_ok = snap.suit_pressure_kpa.unwrap_or(29.6) >= 25.0;
                    let zc = if suit_ok { OUTLINE_VIEW } else { OUTLINE_ALERT };
                    let suit_sz = (rem_h * 0.90).min(ui.available_width() * 0.36).clamp(40.0, 240.0);
                    ui_components::suit_schematic(
                        ui,
                        suit_sz,
                        [INK_PRIMARY, zc, OUTLINE_VIEW, INK_PRIMARY, zc],
                    );
                    ui.add_space(gap);
                    let sleep_txt = snap
                        .sleep_epochs
                        .first()
                        .map(|e| format!("{} ({:.0} BPM)", e.sleep_state, e.hr_bpm_avg.unwrap_or(56.0)))
                        .unwrap_or_else(|| "SLEEP NOMINAL".to_string());
                    ui.vertical(|ui| {
                        ui_components::stat_card(
                            ui,
                            rem_h.max(48.0),
                            &format!("ACTIGRAPHY (TARGET {}M)", snap.sleep_target_min),
                            &sleep_txt,
                            INK_PRIMARY,
                            theme,
                        );
                    });
                });
            });
        });
    } else {
        // INPUT MODE: Glove-friendly ISS Food Cache selector + Meal % + Log Meal / Hydration / Exercise
        let avail_w = ui.available_width();
        let avail_h = ui.available_height();
        let narrow = avail_w < 540.0;
        let gap = (avail_h * 0.018).clamp(4.0, 10.0);
        let hdr_sz = (avail_h * 0.042).min(avail_w * 0.030).clamp(11.0, 24.0);
        let food_count = app.snapshot.food_cache.len();
        let selected_idx = app.selected_food_idx.min(food_count.saturating_sub(1));

        let food_cols = if narrow { 2 } else { 3 };
        let food_rows = food_count.max(1).div_ceil(food_cols);
        let portion_rows = if narrow { 2 } else { 1 };
        let total_rows = (food_rows + portion_rows + 2) as f32;
        let body_h = (avail_h - hdr_sz - gap * (total_rows + 1.0)).max(160.0);
        let btn_h = (body_h / total_rows).clamp(38.0, 110.0);

        ui.label(
            egui::RichText::new(format!(
                "ISS RATION, EXERCISE & HYDRATION  —  {}",
                app.status_banner
            ))
            .size(hdr_sz)
            .strong()
            .color(INK_PRIMARY),
        );
        ui.add_space(gap);

        let foods: Vec<(usize, String, String)> = app
            .snapshot
            .food_cache
            .iter()
            .enumerate()
            .map(|(idx, f)| {
                let short_name = f
                    .item_name
                    .split_whitespace()
                    .take(2)
                    .collect::<Vec<_>>()
                    .join(" ");
                (
                    idx,
                    format!("{} {}", f.food_id, short_name),
                    format!("{:.0}K|{:.0}NA", f.kcal, f.sodium_mg),
                )
            })
            .collect();

        for chunk in foods.chunks(food_cols) {
            ui.columns(food_cols, |cols| {
                for (c, (idx, left, right)) in chunk.iter().enumerate() {
                    let w = cols[c].available_width();
                    if ui_components::touch_button(
                        &mut cols[c],
                        &ctx,
                        ("food_select", *idx),
                        Vec2::new(w, btn_h),
                        left,
                        right,
                        selected_idx == *idx,
                        CardTheme::INPUT,
                    )
                    .clicked()
                    {
                        app.selected_food_idx = *idx;
                    }
                }
            });
            ui.add_space(gap);
        }

        // Consumption percentage buttons (100%, 50%, 25%, 0%)
        let pcts = [100_u8, 50, 25, 0];
        if narrow {
            for r in 0..2 {
                ui.columns(2, |cols| {
                    for c in 0..2 {
                        let pct = pcts[r * 2 + c];
                        let w = cols[c].available_width();
                        let label = format!("PORTION {}%", pct);
                        if ui_components::touch_button(
                            &mut cols[c],
                            &ctx,
                            ("meal_pct_btn", pct),
                            Vec2::new(w, btn_h),
                            &label,
                            "",
                            app.selected_meal_pct == pct,
                            CardTheme::INPUT,
                        )
                        .clicked()
                        {
                            app.selected_meal_pct = pct;
                        }
                    }
                });
                ui.add_space(gap);
            }
        } else {
            ui.columns(4, |cols| {
                for (i, pct) in pcts.iter().enumerate() {
                    let w = cols[i].available_width();
                    let label = format!("PORTION {}%", pct);
                    if ui_components::touch_button(
                        &mut cols[i],
                        &ctx,
                        ("meal_pct_btn", *pct),
                        Vec2::new(w, btn_h),
                        &label,
                        "",
                        app.selected_meal_pct == *pct,
                        CardTheme::INPUT,
                    )
                    .clicked()
                    {
                        app.selected_meal_pct = *pct;
                    }
                }
            });
            ui.add_space(gap);
        }

        // Action commit buttons: Meal + Hydration Pouch + Exercise
        ui.columns(3, |cols| {
            let w0 = cols[0].available_width();
            if ui_components::touch_button(
                &mut cols[0],
                &ctx,
                "commit_meal_log",
                Vec2::new(w0, btn_h),
                if narrow { "LOG MEAL" } else { "LOG SELECTED MEAL" },
                &format!("{}%", app.selected_meal_pct),
                false,
                CardTheme::INPUT,
            )
            .clicked()
            {
                app.log_selected_meal();
            }

            let w1 = cols[1].available_width();
            if ui_components::touch_button(
                &mut cols[1],
                &ctx,
                "commit_fluid_log",
                Vec2::new(w1, btn_h),
                "+250ML POUCH",
                "FLUID",
                false,
                CardTheme::INPUT,
            )
            .clicked()
            {
                app.log_hydration_pouch();
            }

            let w2 = cols[2].available_width();
            if ui_components::touch_button(
                &mut cols[2],
                &ctx,
                "commit_exercise_log",
                Vec2::new(w2, btn_h),
                "+15M EXERCISE",
                "ARED",
                false,
                CardTheme::INPUT,
            )
            .clicked()
            {
                app.log_exercise_session("ARED-15M", "15M ARED RESISTANCE");
            }
        });
        ui.add_space(gap);

        let totals_txt = format!(
            "{:.0}/{:.0} KCAL | {:.0}MG NA | {:.0} ML",
            app.snapshot.daily_nutrition.kcal_total,
            app.snapshot.daily_physio.kcal_burned_total,
            app.snapshot.daily_nutrition.sodium_mg_total,
            app.snapshot.daily_nutrition.fluid_ml_total
        );
        let recent_txt = if app.snapshot.recent_inputs.is_empty() {
            "NO LOGS IN WAL".to_string()
        } else {
            app.snapshot
                .recent_inputs
                .iter()
                .take(2)
                .map(|inp| {
                    format!(
                        "{} ({}%)",
                        inp.reference_code.as_deref().unwrap_or(&inp.input_type),
                        inp.consumption_pct.unwrap_or(100)
                    )
                })
                .collect::<Vec<_>>()
                .join(" | ")
        };

        if narrow {
            ui_components::stat_card(
                ui,
                btn_h,
                "IN / BURN / SODIUM / FLUID",
                &totals_txt,
                INK_PRIMARY,
                CardTheme::VIEW,
            );
        } else {
            ui.columns(2, |cols| {
                ui_components::stat_card(
                    &mut cols[0],
                    btn_h,
                    "TODAY INTAKE / BURN / SODIUM / FLUID",
                    &totals_txt,
                    INK_PRIMARY,
                    CardTheme::VIEW,
                );
                ui_components::stat_card(
                    &mut cols[1],
                    btn_h,
                    "RECENT CREW LOGS (SQLITE WAL)",
                    &recent_txt,
                    INK_PRIMARY,
                    CardTheme::VIEW,
                );
            });
        }
    }
}

fn render_hub_nav_detail(ui: &mut egui::Ui, app: &mut BiomonApp, theme: CardTheme, is_alert: bool) {
    let ctx = ui.ctx().clone();
    let val_col = if is_alert { OUTLINE_ALERT } else { INK_PRIMARY };
    let avail_w = ui.available_width();
    let avail_h = ui.available_height();
    let narrow = avail_w < 540.0;
    let gap = (avail_h * 0.022).clamp(4.0, 12.0);
    let hdr_sz = (avail_h * 0.042).min(avail_w * 0.03).clamp(12.0, 26.0);
    let top_h = if app.mode == AppMode::ViewMode {
        ((avail_h - gap * 2.0) / if narrow { 4.0 } else { 3.0 }).max(44.0)
    } else {
        let total_rows = if narrow { 6.0 } else { 3.0 };
        ((avail_h - hdr_sz - gap * total_rows) / total_rows).max(48.0)
    };

    let nav_cards: [(&str, String, Color32); 4] = [
        ("HABITAT RANGE", format!("{} M", app.snapshot.hub_distance_m), val_col),
        (
            "HEADING / CARDINAL",
            format!("{:03.0}° {}", app.snapshot.heading_deg, app.snapshot.cardinal_heading()),
            INK_PRIMARY,
        ),
        (
            "PHASE / MISSION SOL",
            format!("{} / SOL {:02}", app.snapshot.active_phase, app.snapshot.mission_day),
            INK_PRIMARY,
        ),
        (
            "G-LOAD / IMU ACT",
            format!("{:.2} G / {:.2}", app.snapshot.g_load, app.snapshot.imu_activity_idx),
            INK_PRIMARY,
        ),
    ];

    if narrow {
        for r in 0..2 {
            ui.columns(2, |cols| {
                for c in 0..2 {
                    let (lbl, val, col) = &nav_cards[r * 2 + c];
                    ui_components::stat_card(&mut cols[c], top_h, lbl, val, *col, theme);
                }
            });
            ui.add_space(gap);
        }
    } else {
        ui.columns(4, |cols| {
            for c in 0..4 {
                let (lbl, val, col) = &nav_cards[c];
                ui_components::stat_card(&mut cols[c], top_h, lbl, val, *col, theme);
            }
        });
        ui.add_space(gap);
    }

    if app.mode == AppMode::ViewMode {
        let rem_h = (avail_h - (top_h + gap) * if narrow { 2.0 } else { 1.0 }).max(96.0);
        let pair_h = ((rem_h - gap) * 0.5).max(44.0);
        let compass_sz = (rem_h * 0.92).min(avail_w * 0.24).clamp(54.0, 280.0);

        ui.columns(2, |cols| {
            cols[0].horizontal(|ui| {
                ui_components::compass_dial(
                    ui,
                    compass_sz,
                    app.snapshot.heading_deg,
                    app.snapshot.target_bearing_deg,
                    if is_alert { OUTLINE_ALERT } else { OUTLINE_INPUT },
                );
                ui.add_space(gap);
                ui.vertical(|ui| {
                    ui_components::stat_card(
                        ui,
                        pair_h,
                        "CREW PROFILE",
                        &format!(
                            "{} ({:.0}KG {:.0}CM)",
                            app.snapshot.callsign, app.snapshot.weight_kg, app.snapshot.height_cm
                        ),
                        INK_PRIMARY,
                        theme,
                    );
                    ui.add_space(gap);
                    ui_components::stat_card(
                        ui,
                        pair_h,
                        "CABIN MOTION / TEMP",
                        &format!(
                            "IDX {:.2} / {:.1}C",
                            app.snapshot.cabin_motion_idx,
                            app.snapshot.cabin_temp_c.unwrap_or(21.5)
                        ),
                        INK_PRIMARY,
                        theme,
                    );
                });
            });

            ui_components::stat_card(
                &mut cols[1],
                pair_h,
                "EARTH–MARS EPHEMERIS",
                &format!(
                    "{:.2} AU  |  {:04.1} MIN LIGHT",
                    app.snapshot.earth_mars_au, app.snapshot.one_way_light_min
                ),
                INK_PRIMARY,
                theme,
            );
            cols[1].add_space(gap);
            ui_components::stat_card(
                &mut cols[1],
                pair_h,
                "TELEMETRY SEQ / UTC",
                &format!(
                    "#{}  {}",
                    app.snapshot.seq_id,
                    app.snapshot
                        .recorded_at_utc
                        .strip_prefix("2026-")
                        .unwrap_or(&app.snapshot.recorded_at_utc)
                ),
                INK_PRIMARY,
                theme,
            );
        });
    } else {
        // INPUT MODE: Mission Phase Switcher + Heading & Range simulation controls
        ui.label(
            egui::RichText::new("GLOVE FLIGHT PHASE & NAVIGATION CONTROLS")
                .size(hdr_sz)
                .strong()
                .color(INK_PRIMARY),
        );
        ui.add_space(gap);

        let phases = ["LAUNCH", "ORBIT", "EVA", "REENTRY"];
        if narrow {
            for r in 0..2 {
                ui.columns(2, |cols| {
                    for c in 0..2 {
                        let phase = phases[r * 2 + c];
                        let w = cols[c].available_width();
                        let active = app.snapshot.active_phase.eq_ignore_ascii_case(phase);
                        if ui_components::touch_button(
                            &mut cols[c],
                            &ctx,
                            ("nav_phase_btn", phase),
                            Vec2::new(w, top_h),
                            phase,
                            if active { "ACTIVE" } else { "" },
                            active,
                            CardTheme::INPUT,
                        )
                        .clicked()
                        {
                            app.set_mission_phase(phase);
                        }
                    }
                });
                ui.add_space(gap);
            }
        } else {
            ui.columns(4, |cols| {
                for (i, phase) in phases.iter().enumerate() {
                    let w = cols[i].available_width();
                    let active = app.snapshot.active_phase.eq_ignore_ascii_case(phase);
                    if ui_components::touch_button(
                        &mut cols[i],
                        &ctx,
                        ("nav_phase_btn", *phase),
                        Vec2::new(w, top_h),
                        phase,
                        if active { "ACTIVE" } else { "" },
                        active,
                        CardTheme::INPUT,
                    )
                    .clicked()
                    {
                        app.set_mission_phase(phase);
                    }
                }
            });
            ui.add_space(gap);
        }

        if narrow {
            ui.columns(2, |cols| {
                let w0 = cols[0].available_width();
                if ui_components::touch_button(
                    &mut cols[0],
                    &ctx,
                    "nav_hdg_left",
                    Vec2::new(w0, top_h),
                    "HDG -45°",
                    "",
                    false,
                    CardTheme::INPUT,
                )
                .clicked()
                {
                    app.snapshot.heading_deg = (app.snapshot.heading_deg - 45.0).rem_euclid(360.0);
                }

                let w1 = cols[1].available_width();
                if ui_components::touch_button(
                    &mut cols[1],
                    &ctx,
                    "nav_hdg_right",
                    Vec2::new(w1, top_h),
                    "HDG +45°",
                    "",
                    false,
                    CardTheme::INPUT,
                )
                .clicked()
                {
                    app.snapshot.heading_deg = (app.snapshot.heading_deg + 45.0).rem_euclid(360.0);
                }
            });
            ui.add_space(gap);
            ui.columns(2, |cols| {
                let w2 = cols[0].available_width();
                if ui_components::touch_button(
                    &mut cols[0],
                    &ctx,
                    "nav_range_plus",
                    Vec2::new(w2, top_h),
                    "RANGE +250M",
                    "",
                    false,
                    CardTheme::INPUT,
                )
                .clicked()
                {
                    app.snapshot.hub_distance_m = (app.snapshot.hub_distance_m + 250).min(3000);
                    app.snapshot.recompute_derived();
                }

                let w3 = cols[1].available_width();
                if ui_components::touch_button(
                    &mut cols[1],
                    &ctx,
                    "nav_range_home",
                    Vec2::new(w3, top_h),
                    "HUB 222M",
                    "",
                    false,
                    CardTheme::INPUT,
                )
                .clicked()
                {
                    app.snapshot.hub_distance_m = 222;
                    app.snapshot.recompute_derived();
                }
            });
        } else {
            ui.columns(4, |cols| {
                let w0 = cols[0].available_width();
                if ui_components::touch_button(
                    &mut cols[0],
                    &ctx,
                    "nav_hdg_left",
                    Vec2::new(w0, top_h),
                    "HEADING -45°",
                    "",
                    false,
                    CardTheme::INPUT,
                )
                .clicked()
                {
                    app.snapshot.heading_deg = (app.snapshot.heading_deg - 45.0).rem_euclid(360.0);
                }

                let w1 = cols[1].available_width();
                if ui_components::touch_button(
                    &mut cols[1],
                    &ctx,
                    "nav_hdg_right",
                    Vec2::new(w1, top_h),
                    "HEADING +45°",
                    "",
                    false,
                    CardTheme::INPUT,
                )
                .clicked()
                {
                    app.snapshot.heading_deg = (app.snapshot.heading_deg + 45.0).rem_euclid(360.0);
                }

                let w2 = cols[2].available_width();
                if ui_components::touch_button(
                    &mut cols[2],
                    &ctx,
                    "nav_range_plus",
                    Vec2::new(w2, top_h),
                    "RANGE +250 M",
                    "",
                    false,
                    CardTheme::INPUT,
                )
                .clicked()
                {
                    app.snapshot.hub_distance_m = (app.snapshot.hub_distance_m + 250).min(3000);
                    app.snapshot.recompute_derived();
                }

                let w3 = cols[3].available_width();
                if ui_components::touch_button(
                    &mut cols[3],
                    &ctx,
                    "nav_range_home",
                    Vec2::new(w3, top_h),
                    "RETURN HUB 222M",
                    "",
                    false,
                    CardTheme::INPUT,
                )
                .clicked()
                {
                    app.snapshot.hub_distance_m = 222;
                    app.snapshot.recompute_derived();
                }
            });
        }
    }
}

fn render_comms_detail(ui: &mut egui::Ui, app: &mut BiomonApp, theme: CardTheme, is_alert: bool) {
    let ctx = ui.ctx().clone();
    let val_col = if is_alert { OUTLINE_ALERT } else { INK_PRIMARY };
    let avail_w = ui.available_width();
    let avail_h = ui.available_height();
    let narrow = avail_w < 540.0;
    let gap = (avail_h * 0.022).clamp(4.0, 12.0);
    let rows_total = if narrow {
        if app.mode == AppMode::ViewMode { 4.0 } else { 6.0 }
    } else {
        3.0
    };
    let row_h = ((avail_h - gap * (rows_total - 1.0)) / rows_total).max(44.0);

    let top_cards: [(&str, String, Color32); 4] = [
        ("SURFACE LAN LINK", app.snapshot.comms_state.label().to_string(), val_col),
        ("HUB ENDPOINT", app.lan_client.target_hub_addr().to_string(), INK_PRIMARY),
        (
            "UNSYNCED RECORDS",
            format!("{} TOTAL ({} INP)", app.snapshot.unsynced.total(), app.snapshot.unsynced.crew_inputs),
            INK_PRIMARY,
        ),
        (
            "ONE-WAY LIGHT DELAY",
            format!("{:04.1} MIN", app.snapshot.one_way_light_min),
            INK_PRIMARY,
        ),
    ];

    if narrow {
        for r in 0..2 {
            ui.columns(2, |cols| {
                for c in 0..2 {
                    let (lbl, val, col) = &top_cards[r * 2 + c];
                    ui_components::stat_card(&mut cols[c], row_h, lbl, val, *col, theme);
                }
            });
            ui.add_space(gap);
        }
    } else {
        ui.columns(4, |cols| {
            for c in 0..4 {
                let (lbl, val, col) = &top_cards[c];
                ui_components::stat_card(&mut cols[c], row_h, lbl, val, *col, theme);
            }
        });
        ui.add_space(gap);
    }

    // Row 2: Queue breakdown + Signal Bars + Recent Crew Input Queue
    let sig_sz = (row_h * 0.85).min(avail_w * 0.16).clamp(32.0, 160.0);
    ui.columns(2, |cols| {
        cols[0].horizontal(|ui| {
            ui_components::signal_bars(
                ui,
                sig_sz,
                app.snapshot.comms_state == ConnectionState::Online,
            );
            ui.add_space(gap);
            ui.vertical(|ui| {
                ui_components::stat_card(
                    ui,
                    row_h,
                    "RING BUFFER / SLEEP / DAILY",
                    &format!(
                        "{} TEL | {} SLP | {} DLY",
                        app.snapshot.unsynced.telemetry_frames,
                        app.snapshot.unsynced.sleep_epochs,
                        app.snapshot.unsynced.daily_states
                    ),
                    INK_PRIMARY,
                    theme,
                );
            });
        });

        let latest_input_str = app
            .snapshot
            .recent_inputs
            .first()
            .map(|inp| {
                format!(
                    "{} {} ({}%)",
                    inp.input_type,
                    inp.reference_code.as_deref().unwrap_or("-"),
                    inp.consumption_pct.unwrap_or(100)
                )
            })
            .unwrap_or_else(|| "QUEUE EMPTY".to_string());
        ui_components::stat_card(
            &mut cols[1],
            row_h,
            "LATEST LOGGED CREW INPUT",
            &latest_input_str,
            INK_PRIMARY,
            theme,
        );
    });

    ui.add_space(gap);

    if app.mode == AppMode::ViewMode {
        // Pure read-only telemetry row in ViewMode (keeps tap-anywhere return active)
        ui.columns(2, |cols| {
            ui_components::stat_card(
                &mut cols[0],
                row_h,
                "EARTH–MARS DISTANCE",
                &format!("{:.3} AU", app.snapshot.earth_mars_au),
                INK_PRIMARY,
                theme,
            );
            ui_components::stat_card(
                &mut cols[1],
                row_h,
                "FRAMES TRANSMITTED",
                &format!("{} PKTS", app.lan_client.frames_sent()),
                INK_PRIMARY,
                theme,
            );
        });
    } else if narrow {
        // Row 3 (narrow): 2 buttons + sim controls + stat card
        ui.columns(2, |cols| {
            let w0 = cols[0].available_width();
            if ui_components::touch_button(
                &mut cols[0],
                &ctx,
                "comms_flush_btn",
                Vec2::new(w0, row_h),
                "FLUSH & SYNC",
                &format!("{} REC", app.snapshot.unsynced.total()),
                false,
                CardTheme::INPUT,
            )
            .clicked()
            {
                app.sync_queued_packets();
            }

            let w1 = cols[1].available_width();
            let manual_blackout = app.lan_client.is_manual_blackout();
            let toggle_lbl = if manual_blackout {
                "RESTORE LAN"
            } else {
                "BLACKOUT"
            };
            if ui_components::touch_button(
                &mut cols[1],
                &ctx,
                "comms_toggle_link",
                Vec2::new(w1, row_h),
                toggle_lbl,
                "",
                manual_blackout,
                CardTheme::INPUT,
            )
            .clicked()
            {
                let new_state = app.lan_client.toggle_connection_state();
                app.snapshot.comms_state = new_state;
                app.snapshot.recompute_derived();
                app.status_banner = format!("LAN STATE: {}", new_state.label());
            }
        });
        ui.add_space(gap);
        ui.columns(2, |cols| {
            let w0 = cols[0].available_width();
            if ui_components::touch_button(
                &mut cols[0],
                &ctx,
                "comms_sim_scenario_btn",
                Vec2::new(w0, row_h),
                "SIM SCENARIO",
                app.sim.scenario.label(),
                false,
                CardTheme::INPUT,
            )
            .clicked()
            {
                app.cycle_sim_scenario();
            }

            let w1 = cols[1].available_width();
            if ui_components::touch_button(
                &mut cols[1],
                &ctx,
                "comms_sim_toggle_btn",
                Vec2::new(w1, row_h),
                "1HZ SIM",
                if app.sim.enabled { "ON" } else { "PAUSED" },
                app.sim.enabled,
                CardTheme::INPUT,
            )
            .clicked()
            {
                app.toggle_sim_enabled();
            }
        });
        ui.add_space(gap);
        ui_components::stat_card(
            ui,
            row_h,
            "FRAMES TRANSMITTED",
            &format!("{} PKTS", app.lan_client.frames_sent()),
            INK_PRIMARY,
            theme,
        );
    } else {
        // Row 3: Interactive Sync / Link Test / Sim controls in InputMode
        ui.columns(4, |cols| {
            let w0 = cols[0].available_width();
            if ui_components::touch_button(
                &mut cols[0],
                &ctx,
                "comms_flush_btn",
                Vec2::new(w0, row_h),
                "FLUSH & SYNC",
                &format!("{} REC", app.snapshot.unsynced.total()),
                false,
                CardTheme::INPUT,
            )
            .clicked()
            {
                app.sync_queued_packets();
            }

            let w1 = cols[1].available_width();
            let manual_blackout = app.lan_client.is_manual_blackout();
            let toggle_lbl = if manual_blackout {
                "RESTORE LAN LINK"
            } else {
                "SIMULATE BLACKOUT"
            };
            if ui_components::touch_button(
                &mut cols[1],
                &ctx,
                "comms_toggle_link",
                Vec2::new(w1, row_h),
                toggle_lbl,
                "",
                manual_blackout,
                CardTheme::INPUT,
            )
            .clicked()
            {
                let new_state = app.lan_client.toggle_connection_state();
                app.snapshot.comms_state = new_state;
                app.snapshot.recompute_derived();
                app.status_banner = format!("LAN STATE: {}", new_state.label());
            }

            let w2 = cols[2].available_width();
            if ui_components::touch_button(
                &mut cols[2],
                &ctx,
                "comms_sim_toggle_btn",
                Vec2::new(w2, row_h),
                if app.sim.enabled { "1HZ SIM: ON" } else { "1HZ SIM: OFF" },
                app.sim.scenario.label(),
                app.sim.enabled,
                CardTheme::INPUT,
            )
            .clicked()
            {
                app.toggle_sim_enabled();
            }

            ui_components::stat_card(
                &mut cols[3],
                row_h,
                "FRAMES TRANSMITTED",
                &format!("{} PKTS", app.lan_client.frames_sent()),
                INK_PRIMARY,
                theme,
            );
        });
    }
}

fn render_directive_card(
    ui: &mut egui::Ui,
    height: f32,
    directive: &str,
    theme: CardTheme,
    is_alert: bool,
) {
    let h = height.max(48.0);
    let w = ui.available_width().max(48.0);
    let rect = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover()).0;
    ui.painter().rect(
        rect,
        CornerRadius::same(10),
        theme.bg,
        Stroke::new(theme.stroke_width, theme.border),
        StrokeKind::Inside,
    );
    let pad_x = (rect.width() * 0.03).clamp(8.0, 18.0);
    let pad_y = (rect.height() * 0.12).clamp(6.0, 14.0);
    let inner = rect.shrink2(Vec2::new(pad_x, pad_y));
    let lbl_sz = (inner.height() * 0.24).min(inner.width() * 0.04).clamp(10.0, 20.0);
    let val_sz = (inner.height() * 0.42).min(inner.width() * 0.048).clamp(12.0, 28.0);
    ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
        ui.label(
            egui::RichText::new("ACTIVE PROTOCOL DIRECTIVE")
                .size(lbl_sz)
                .color(INK_PRIMARY),
        );
        ui.add_space((inner.height() * 0.06).clamp(2.0, 8.0));
        ui.add(
            egui::Label::new(
                egui::RichText::new(directive)
                    .size(val_sz)
                    .strong()
                    .monospace()
                    .color(if is_alert { OUTLINE_ALERT } else { INK_PRIMARY }),
            )
            .wrap(),
        );
    });
}

fn render_hazard_detail(ui: &mut egui::Ui, app: &mut BiomonApp, theme: CardTheme, is_alert: bool) {
    let ctx = ui.ctx().clone();
    let val_col = if is_alert { OUTLINE_ALERT } else { INK_PRIMARY };
    let avail_w = ui.available_width();
    let avail_h = ui.available_height();
    let narrow = avail_w < 540.0;
    let gap = (avail_h * 0.022).clamp(4.0, 12.0);
    let card_rows = match (app.mode == AppMode::ViewMode, narrow) {
        (true, true) => 3.0,
        (true, false) => 2.0,
        (false, true) => 6.0,
        (false, false) => 4.0,
    };
    let rows_total = card_rows + 0.85;
    let row_h = ((avail_h - gap * card_rows) / rows_total).max(42.0);
    let dir_h = (avail_h - (row_h + gap) * card_rows).max(48.0);

    let sans_str = app
        .snapshot
        .sans_state
        .as_ref()
        .map(|s| format!("{} ({:.0})", s.sans_risk, s.sans_score))
        .unwrap_or_else(|| "LOW (24)".to_string());

    let top_cards: [(&str, String, Color32); 4] = [
        ("RADIATION FLUX", format!("{:.2} USV/H", app.snapshot.radiation_usv_hr), val_col),
        (
            "CABIN CO2 / O2",
            format!("{:.0} PPM / {:.1}%", app.snapshot.cabin_co2_ppm, app.snapshot.cabin_o2_pct.unwrap_or(20.9)),
            val_col,
        ),
        (
            "CABIN P / HUMID",
            format!(
                "{:.1} KPA / {:.0}%",
                app.snapshot.cabin_pressure_kpa.unwrap_or(101.3),
                app.snapshot.cabin_humidity_pct.unwrap_or(44.0)
            ),
            INK_PRIMARY,
        ),
        ("SANS RISK TIER", sans_str, INK_PRIMARY),
    ];

    if narrow {
        for r in 0..2 {
            ui.columns(2, |cols| {
                for c in 0..2 {
                    let (lbl, val, col) = &top_cards[r * 2 + c];
                    ui_components::stat_card(&mut cols[c], row_h, lbl, val, *col, theme);
                }
            });
            ui.add_space(gap);
        }
    } else {
        ui.columns(4, |cols| {
            for c in 0..4 {
                let (lbl, val, col) = &top_cards[c];
                ui_components::stat_card(&mut cols[c], row_h, lbl, val, *col, theme);
            }
        });
        ui.add_space(gap);
    }

    let proto_count = app.snapshot.protocols.len();
    let sel_idx = app.selected_protocol_idx.min(proto_count.saturating_sub(1));
    let unack = app
        .snapshot
        .local_alerts
        .iter()
        .filter(|a| a.acknowledged == 0)
        .count();

    if app.mode == AppMode::ViewMode {
        // Pure read-only view (tap anywhere returns to HUD)
        if let Some(proto) = app.snapshot.protocols.get(sel_idx).cloned() {
            ui.columns(2, |cols| {
                ui_components::stat_card(
                    &mut cols[0],
                    row_h,
                    &format!("{} ({})", proto.protocol_id, proto.applicable_phase),
                    &proto.hazard_category,
                    INK_PRIMARY,
                    theme,
                );
                ui_components::stat_card(
                    &mut cols[1],
                    row_h,
                    "N2 R-INDEX / OPEN ALERTS",
                    &format!("R={:.2} | {} OPEN", app.snapshot.blood_n2_idx.unwrap_or(1.08), unack),
                    if unack > 0 { OUTLINE_ALERT } else { INK_PRIMARY },
                    theme,
                );
            });
            ui.add_space(gap);
            render_directive_card(ui, dir_h, &proto.immediate_action, theme, is_alert);
        }
        return;
    }

    // INPUT MODE: Interactive protocol selectors + O2 prebreathe + Ack Alerts
    let proto_cats: Vec<(usize, String)> = app
        .snapshot
        .protocols
        .iter()
        .enumerate()
        .map(|(i, p)| (i, p.hazard_category.clone()))
        .collect();

    if !proto_cats.is_empty() {
        ui.columns(proto_cats.len(), |cols| {
            for (i, cat) in &proto_cats {
                let w = cols[*i].available_width();
                if ui_components::touch_button(
                    &mut cols[*i],
                    &ctx,
                    ("proto_sel_btn", *i),
                    Vec2::new(w, row_h),
                    cat,
                    "",
                    sel_idx == *i,
                    CardTheme::INPUT,
                )
                .clicked()
                {
                    app.selected_protocol_idx = *i;
                }
            }
        });
    }

    ui.add_space(gap);

    if let Some(proto) = app.snapshot.protocols.get(sel_idx).cloned() {
        if narrow {
            ui.columns(2, |cols| {
                let w1 = cols[0].available_width();
                if ui_components::touch_button(
                    &mut cols[0],
                    &ctx,
                    "prebreathe_step_btn",
                    Vec2::new(w1, row_h),
                    "O2 +10M",
                    &format!("R={:.2}", app.snapshot.blood_n2_idx.unwrap_or(1.08)),
                    false,
                    CardTheme::INPUT,
                )
                .clicked()
                {
                    app.step_prebreathe_washout(600.0);
                }

                let w2 = cols[1].available_width();
                if ui_components::touch_button(
                    &mut cols[1],
                    &ctx,
                    "ack_local_alerts_btn",
                    Vec2::new(w2, row_h),
                    "ACK ALERTS",
                    &format!("{} OPEN", unack),
                    unack > 0,
                    CardTheme::INPUT,
                )
                .clicked()
                {
                    app.acknowledge_alerts();
                }
            });
            ui.add_space(gap);
            ui_components::stat_card(
                ui,
                row_h,
                &format!("{} ({})", proto.protocol_id, proto.applicable_phase),
                &proto.hazard_category,
                INK_PRIMARY,
                theme,
            );
        } else {
            ui.columns(3, |cols| {
                ui_components::stat_card(
                    &mut cols[0],
                    row_h,
                    &format!("{} ({})", proto.protocol_id, proto.applicable_phase),
                    &proto.hazard_category,
                    INK_PRIMARY,
                    theme,
                );

                let w1 = cols[1].available_width();
                if ui_components::touch_button(
                    &mut cols[1],
                    &ctx,
                    "prebreathe_step_btn",
                    Vec2::new(w1, row_h),
                    "O2 PREBREATHE +10M",
                    &format!("R={:.2}", app.snapshot.blood_n2_idx.unwrap_or(1.08)),
                    false,
                    CardTheme::INPUT,
                )
                .clicked()
                {
                    app.step_prebreathe_washout(600.0);
                }

                let w2 = cols[2].available_width();
                if ui_components::touch_button(
                    &mut cols[2],
                    &ctx,
                    "ack_local_alerts_btn",
                    Vec2::new(w2, row_h),
                    "ACK LOCAL ALERTS",
                    &format!("{} OPEN", unack),
                    unack > 0,
                    CardTheme::INPUT,
                )
                .clicked()
                {
                    app.acknowledge_alerts();
                }
            });
        }

        ui.add_space(gap);
        ui.columns(2, |cols| {
            let w0 = cols[0].available_width();
            if ui_components::touch_button(
                &mut cols[0],
                &ctx,
                "hazard_sim_scenario_btn",
                Vec2::new(w0, row_h),
                "SIM SCENARIO",
                app.sim.scenario.label(),
                false,
                CardTheme::INPUT,
            )
            .clicked()
            {
                app.cycle_sim_scenario();
            }

            let w1 = cols[1].available_width();
            if ui_components::touch_button(
                &mut cols[1],
                &ctx,
                "hazard_sim_toggle_btn",
                Vec2::new(w1, row_h),
                "1HZ BIO SIM",
                if app.sim.enabled { "RUNNING" } else { "PAUSED" },
                app.sim.enabled,
                CardTheme::INPUT,
            )
            .clicked()
            {
                app.toggle_sim_enabled();
            }
        });

        ui.add_space(gap);
        render_directive_card(ui, dir_h, &proto.immediate_action, theme, is_alert);
    }
}
