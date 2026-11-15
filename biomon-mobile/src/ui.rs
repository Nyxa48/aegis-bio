use crate::app::{AppMode, BiomonApp, QuadrantId, Screen};
use crate::calc;
use crate::icons;
use crate::net::ConnectionState;
use crate::ui_components::{self, CardTheme, StatusLevel, palette};
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
    visuals.faint_bg_color = palette::TRACK_BG;
    visuals.hyperlink_color = OUTLINE_VIEW;
    visuals.selection.bg_fill = BG_ACTIVE;
    visuals.selection.stroke = Stroke::new(1.0, INK_PRIMARY);
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
        .frame(egui::Frame::NONE.fill(palette::BG_OUTER))
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
        let border = if mode == AppMode::InputMode {
            OUTLINE_INPUT
        } else {
            palette::BORDER_VIEW
        };
        (border, BORDER_NOMINAL)
    }
}

fn render_main_hud(ui: &mut egui::Ui, app: &mut BiomonApp) {
    let screen_rect = ui.max_rect();
    let (_, response) = ui.allocate_exact_size(screen_rect.size(), Sense::click());
    let painter = ui.painter_at(screen_rect);
    let time_sec = ui.input(|i| i.time);

    painter.rect_filled(screen_rect, CornerRadius::ZERO, palette::BG_OUTER);

    let center = screen_rect.center();
    let circle_radius = (screen_rect.height().min(screen_rect.width()) * 0.19).clamp(95.0, 145.0);

    // DESIGN RULES: Outer window margin 6.0 px, gap between quadrants 8.0 px
    let outer_pad = 6.0;
    let gap = 8.0;
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
    let pad_x = 22.0;
    let pad_y = 16.0;
    let q_h = tl_rect.height();
    let q_w = tl_rect.width();

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
        "BİYOMETRİ",
        FontId::new(32.0, FontFamily::Proportional),
        INK_PRIMARY,
    );

    let heart_y = tl_rect.min.y + pad_y + (q_h * 0.24).clamp(64.0, 86.0);
    let heart_center = Pos2::new(tl_rect.min.x + pad_x + 24.0, heart_y);
    let bpm_u16 = snap.hr_bpm.round().clamp(30.0, 240.0) as u16;
    icons::draw_heart_icon(
        &painter,
        heart_center,
        22.0,
        health_ink,
        bpm_u16,
        time_sec,
        true,
    );
    painter.text(
        Pos2::new(heart_center.x + 38.0, heart_center.y),
        Align2::LEFT_CENTER,
        format!("{:.0} BPM", snap.hr_bpm),
        FontId::new(56.0, FontFamily::Monospace),
        health_ink,
    );

    let suit_p = snap.suit_pressure_kpa.unwrap_or(29.6);
    let suit_ok = suit_p >= 25.0;
    let row2_y = heart_y + (q_h * 0.16).clamp(44.0, 56.0);
    painter.text(
        Pos2::new(tl_rect.min.x + pad_x, row2_y),
        Align2::LEFT_TOP,
        format!(
            "SKB {:.0}  |  SpO₂ {:.0}%  |  {:.1} kPa",
            snap.sbp_mmhg, snap.spo2_pct, suit_p
        ),
        FontId::new(28.0, FontFamily::Monospace),
        INK_PRIMARY,
    );

    let row3_y = row2_y + (q_h * 0.13).clamp(36.0, 46.0);
    let droplet_c = Pos2::new(tl_rect.min.x + pad_x + 14.0, row3_y + 16.0);
    icons::draw_droplet_icon(&painter, droplet_c, 15.0, INK_PRIMARY);
    painter.text(
        Pos2::new(droplet_c.x + 24.0, row3_y),
        Align2::LEFT_TOP,
        format!(
            "{:.0} mL/sa  |  Şİ {:.2} {}",
            snap.sweat_ml_per_sec * 3600.0,
            snap.shock_index,
            snap.orthostatic_risk
        ),
        FontId::new(27.0, FontFamily::Monospace),
        INK_PRIMARY,
    );

    // Safe HR corridor bar near bottom-left of TL quadrant (stopping well left of center circle)
    let (hr_lo, hr_hi) = calc::phase_hr_corridor(&snap.active_phase);
    let corridor_w = (q_w - circle_radius - 58.0).clamp(200.0, 360.0);
    let corridor_rect = Rect::from_min_size(
        Pos2::new(tl_rect.min.x + pad_x, tl_rect.max.y - 36.0),
        Vec2::new(corridor_w, 22.0),
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

    // 5-zone spacesuit schematic in top-right of Top-Left quadrant (above center circle)
    let suit_zone_col = if suit_ok { OUTLINE_VIEW } else { OUTLINE_ALERT };
    let suit_h = (q_h * 0.38).clamp(96.0, 128.0);
    let suit_w = suit_h * (110.0 / 130.0);
    let suit_rect = Rect::from_min_size(
        Pos2::new(tl_rect.max.x - suit_w - 18.0, tl_rect.min.y + 14.0),
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

    // Compass dial in upper-left of Top-Right quadrant (above center circle)
    let compass_sz = (q_h * 0.36).clamp(92.0, 122.0);
    let compass_rect = Rect::from_min_size(
        Pos2::new(tr_rect.min.x + 22.0, tr_rect.min.y + 16.0),
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
        "KONUM",
        FontId::new(32.0, FontFamily::Proportional),
        INK_PRIMARY,
    );
    painter.text(
        Pos2::new(tr_right, tr_top + (q_h * 0.15).clamp(44.0, 56.0)),
        Align2::RIGHT_TOP,
        format!("{} M", snap.hub_distance_m),
        FontId::new(58.0, FontFamily::Monospace),
        hub_ink,
    );
    painter.text(
        Pos2::new(tr_right, tr_top + (q_h * 0.36).clamp(108.0, 132.0)),
        Align2::RIGHT_TOP,
        format!(
            "{:03.0}° {}  |  {}",
            snap.heading_deg,
            snap.cardinal_heading(),
            snap.active_phase
        ),
        FontId::new(30.0, FontFamily::Monospace),
        INK_PRIMARY,
    );
    painter.text(
        Pos2::new(tr_right, tr_rect.max.y - 16.0),
        Align2::RIGHT_BOTTOM,
        format!(
            "{}  |  SOL {:02}  |  {:.1} G",
            snap.callsign, snap.mission_day, snap.g_load
        ),
        FontId::new(28.0, FontFamily::Monospace),
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
        palette::NOMINAL
    };

    painter.text(
        Pos2::new(bl_left, bl_top),
        Align2::LEFT_TOP,
        "BAĞLANTI",
        FontId::new(32.0, FontFamily::Proportional),
        INK_PRIMARY,
    );

    painter.text(
        Pos2::new(bl_left, bl_top + (q_h * 0.16).clamp(44.0, 56.0)),
        Align2::LEFT_TOP,
        format!(
            "TEK YÖN {:.1}dk | {:.2}AU",
            snap.one_way_light_min, snap.earth_mars_au
        ),
        FontId::new(28.0, FontFamily::Monospace),
        INK_PRIMARY,
    );

    painter.text(
        Pos2::new(bl_left, bl_top + (q_h * 0.30).clamp(84.0, 102.0)),
        Align2::LEFT_TOP,
        format!(
            "BEKLEYEN {} | İLETİLEN {}",
            snap.unsynced.total(),
            app.lan_client.frames_sent()
        ),
        FontId::new(28.0, FontFamily::Monospace),
        INK_PRIMARY,
    );

    let sig_rect = Rect::from_min_size(Pos2::new(bl_left, bl_bottom - 54.0), Vec2::splat(50.0));
    ui_components::paint_signal_bars(
        &painter,
        sig_rect,
        snap.comms_state == ConnectionState::Online,
        comms_alert,
    );

    painter.text(
        Pos2::new(bl_left + 64.0, bl_bottom),
        Align2::LEFT_BOTTOM,
        snap.comms_state.label(),
        FontId::new(54.0, FontFamily::Monospace),
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
        palette::NOMINAL
    };
    let hazard_status = if hazard_alert { "UYARI" } else { "OLAĞAN" };

    icons::draw_hazard_icon(
        &painter,
        Pos2::new(br_right - 18.0, br_top + 18.0),
        18.0,
        hazard_color,
    );
    painter.text(
        Pos2::new(br_right - 46.0, br_top),
        Align2::RIGHT_TOP,
        "KABİN ORTAMI",
        FontId::new(32.0, FontFamily::Proportional),
        INK_PRIMARY,
    );

    let sans_label = snap
        .sans_state
        .as_ref()
        .map(|s| s.sans_risk.as_str())
        .unwrap_or("LOW");
    painter.text(
        Pos2::new(br_right, br_top + (q_h * 0.16).clamp(44.0, 56.0)),
        Align2::RIGHT_TOP,
        format!("CO₂ {:.0} ppm | SANS {}", snap.cabin_co2_ppm, sans_label),
        FontId::new(28.0, FontFamily::Monospace),
        INK_PRIMARY,
    );

    painter.text(
        Pos2::new(br_right, br_bottom - 48.0),
        Align2::RIGHT_BOTTOM,
        hazard_status,
        FontId::new(54.0, FontFamily::Proportional),
        hazard_color,
    );

    painter.text(
        Pos2::new(br_right, br_bottom),
        Align2::RIGHT_BOTTOM,
        format!(
            "O₂ {:.1}% | {:.2} µSv/sa",
            snap.cabin_o2_pct.unwrap_or(20.9),
            snap.radiation_usv_hr
        ),
        FontId::new(30.0, FontFamily::Monospace),
        hazard_color,
    );

    // -------------------------------------------------------------------------
    // CENTER CIRCLE BUTTON (CYCLE) — Exact center circle layout preserved
    // -------------------------------------------------------------------------
    let circle_border = mode_outline_color(app.mode);
    let circle_fill = panel_fill(hover_in_circle, hover_in_circle && pointer_down);

    painter.circle(
        center,
        circle_radius,
        circle_fill,
        Stroke::new(BORDER_NOMINAL, circle_border),
    );

    painter.text(
        Pos2::new(center.x, center.y - 24.0),
        Align2::CENTER_CENTER,
        "KONTROL",
        FontId::new(34.0, FontFamily::Proportional),
        INK_PRIMARY,
    );
    painter.text(
        Pos2::new(center.x, center.y + 24.0),
        Align2::CENTER_CENTER,
        app.mode.cycle_badge_label(),
        FontId::new(
            (circle_radius * 0.24).clamp(26.0, 34.0),
            FontFamily::Proportional,
        ),
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
    let is_alert = match quadrant {
        QuadrantId::HealthAndSuit => app.snapshot.alerts.health_danger,
        QuadrantId::HubAndNav => app.snapshot.alerts.hub_danger,
        QuadrantId::CommsAndDisplay => app.snapshot.alerts.comms_danger,
        QuadrantId::HazardAndEnv => app.snapshot.alerts.hazard_danger,
    };

    let (border_color, border_width) = quadrant_border(app.mode, is_alert, time_sec);
    let outer_rect = screen_rect.shrink(6.0);
    let theme = CardTheme::for_mobile(app.mode == AppMode::InputMode, is_alert);

    let painter = ui.painter();
    painter.rect(
        outer_rect,
        CornerRadius::same(10),
        BG_PRIMARY,
        Stroke::new(border_width, border_color),
        StrokeKind::Inside,
    );

    let inner_rect = outer_rect.shrink2(Vec2::new(20.0, 14.0));
    ui.scope_builder(egui::UiBuilder::new().max_rect(inner_rect), |ui| {
        let ctx = ui.ctx().clone();

        // Top header bar: Title + Mode/Status Badge + MODE toggle button + BACK button (>= 140x100)
        ui.horizontal(|ui| {
            let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::hover());
            let ip = ui.painter();
            match quadrant {
                QuadrantId::HealthAndSuit => {
                    icons::draw_shield_icon(ip, icon_rect.center(), 20.0, border_color);
                }
                QuadrantId::HubAndNav => {
                    icons::draw_compass_icon(
                        ip,
                        icon_rect.center(),
                        20.0,
                        border_color,
                        app.snapshot.heading_deg,
                    );
                }
                QuadrantId::CommsAndDisplay => {
                    icons::draw_comms_icon(
                        ip,
                        icon_rect.center(),
                        20.0,
                        border_color,
                        app.snapshot.comms_state == ConnectionState::Online,
                    );
                }
                QuadrantId::HazardAndEnv => {
                    icons::draw_hazard_icon(ip, icon_rect.center(), 20.0, border_color);
                }
            }

            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(quadrant.title())
                        .size(40.0)
                        .strong()
                        .color(INK_PRIMARY),
                );
                let status_lvl = if is_alert {
                    StatusLevel::Critical
                } else if app.mode == AppMode::InputMode {
                    StatusLevel::Caution
                } else {
                    StatusLevel::Nominal
                };
                ui_components::status_badge(ui, status_lvl, app.mode.detail_mode_badge(is_alert));
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui_components::touch_button(
                    ui,
                    &ctx,
                    ("detail_back_btn", quadrant as u8),
                    Vec2::new(210.0, 100.0),
                    "ANA EKRAN",
                    "",
                    false,
                    theme,
                )
                .clicked()
                {
                    app.screen = Screen::MainHud;
                }

                ui.add_space(10.0);

                let mode_btn_lbl = match app.mode {
                    AppMode::ViewMode => "MOD: İZLEME",
                    AppMode::InputMode => "MOD: GİRİŞ",
                };
                if ui_components::touch_button(
                    ui,
                    &ctx,
                    ("detail_cycle_mode", quadrant as u8),
                    Vec2::new(220.0, 100.0),
                    mode_btn_lbl,
                    "",
                    app.mode == AppMode::InputMode,
                    CardTheme::for_mobile(app.mode == AppMode::InputMode, false),
                )
                .clicked()
                {
                    app.mode.toggle();
                }
            });
        });

        ui.add_space(10.0);

        match quadrant {
            QuadrantId::HealthAndSuit => render_health_detail(ui, app, theme, is_alert),
            QuadrantId::HubAndNav => render_hub_nav_detail(ui, app, theme, is_alert),
            QuadrantId::CommsAndDisplay => render_comms_detail(ui, app, theme, is_alert),
            QuadrantId::HazardAndEnv => render_hazard_detail(ui, app, theme, is_alert),
        }
    });
}

fn render_health_detail(ui: &mut egui::Ui, app: &mut BiomonApp, theme: CardTheme, is_alert: bool) {
    let ctx = ui.ctx().clone();
    if app.mode == AppMode::ViewMode {
        let snap = &app.snapshot;
        let val_col = if is_alert { OUTLINE_ALERT } else { INK_PRIMARY };

        // Row 1: 4 primary physiological cards
        ui.columns(4, |cols| {
            ui_components::stat_card(
                &mut cols[0],
                116.0,
                "NABIZ",
                &format!("{:.0} BPM", snap.hr_bpm),
                val_col,
                theme,
            );
            ui_components::stat_card(
                &mut cols[1],
                116.0,
                "SKB / SOLUNUM",
                &format!("{:.0} / {:.0}", snap.sbp_mmhg, snap.resp_brpm),
                INK_PRIMARY,
                theme,
            );
            ui_components::stat_card(
                &mut cols[2],
                116.0,
                "SpO₂ / DOKU O₂",
                &format!(
                    "{:.0}% / {:.0}%",
                    snap.spo2_pct,
                    snap.tissue_o2_pct.unwrap_or(93.5)
                ),
                val_col,
                theme,
            );
            ui_components::stat_card(
                &mut cols[3],
                116.0,
                "N2 R-IDX / SHOCK",
                &format!(
                    "{:.2} / {:.2}",
                    snap.blood_n2_idx.unwrap_or(1.08),
                    snap.shock_index
                ),
                val_col,
                theme,
            );
        });

        ui.add_space(10.0);

        // Row 2: Suit + Metabolic + Osteo/Sleep cards
        ui.columns(4, |cols| {
            ui_components::stat_card(
                &mut cols[0],
                116.0,
                "GİYSİ BASINÇ / NEM",
                &format!(
                    "{:.1} KPA / {:.0}%",
                    snap.suit_pressure_kpa.unwrap_or(29.6),
                    snap.suit_humidity_pct.unwrap_or(38.0)
                ),
                INK_PRIMARY,
                theme,
            );
            ui_components::stat_card(
                &mut cols[1],
                116.0,
                "SKIN / BURN RATE",
                &format!(
                    "{:.1}C / {:.0} KCAL/H",
                    snap.skin_temp_c,
                    snap.kcal_per_sec * 3600.0
                ),
                INK_PRIMARY,
                theme,
            );
            ui_components::stat_card(
                &mut cols[2],
                116.0,
                "TERLEME / SIVI ALIMI",
                &format!(
                    "{:.0} ML/H / {:.0} ML",
                    snap.sweat_ml_per_sec * 3600.0,
                    snap.daily_nutrition.fluid_ml_total
                ),
                INK_PRIMARY,
                theme,
            );
            let osteo_risk = snap
                .osteo_state
                .as_ref()
                .map(|o| o.osteo_risk.as_str())
                .unwrap_or("LOW");
            ui_components::stat_card(
                &mut cols[3],
                116.0,
                "OSTEO / ORTHO",
                &format!("{} / {}", osteo_risk, snap.orthostatic_risk),
                INK_PRIMARY,
                theme,
            );
        });

        ui.add_space(10.0);

        // Row 3: HR Corridor bar + Suit Schematic + Daily Nutrition & Sleep readout
        ui.columns(2, |cols| {
            let (lo, hi) = calc::phase_hr_corridor(&snap.active_phase);
            cols[0].label(
                egui::RichText::new(format!(
                    "PHASE {} HR CORRIDOR ({:.0}–{:.0} BPM)  |  EVA {:02}M:{:02}S",
                    snap.active_phase,
                    lo,
                    hi,
                    snap.eva_elapsed_sec / 60,
                    snap.eva_elapsed_sec % 60
                ))
                .size(28.0)
                .strong()
                .color(INK_PRIMARY),
            );
            cols[0].add_space(6.0);
            let col0_w = cols[0].available_width();
            ui_components::corridor_bar(
                &mut cols[0],
                col0_w,
                snap.hr_bpm as f32,
                lo as f32,
                hi as f32,
                if is_alert {
                    OUTLINE_ALERT
                } else {
                    OUTLINE_VIEW
                },
            );
            cols[0].add_space(10.0);
            ui_components::stat_card(
                &mut cols[0],
                116.0,
                "SOL NUTRITION (KCAL / NA / CA)",
                &format!(
                    "{:.0} KCAL | {:.0}MG NA | {:.0}MG CA",
                    snap.daily_nutrition.kcal_total,
                    snap.daily_nutrition.sodium_mg_total,
                    snap.daily_nutrition.calcium_mg_total
                ),
                INK_PRIMARY,
                theme,
            );

            cols[1].horizontal(|ui| {
                let suit_ok = snap.suit_pressure_kpa.unwrap_or(29.6) >= 25.0;
                let zc = if suit_ok { OUTLINE_VIEW } else { OUTLINE_ALERT };
                ui_components::suit_schematic(
                    ui,
                    180.0,
                    [INK_PRIMARY, zc, OUTLINE_VIEW, INK_PRIMARY, zc],
                );
                ui.add_space(12.0);
                let sleep_ep = snap.sleep_epochs.first();
                let sleep_txt = sleep_ep
                    .map(|e| {
                        format!(
                            "{} ({:.0} BPM)",
                            e.sleep_state,
                            e.hr_bpm_avg.unwrap_or(56.0)
                        )
                    })
                    .unwrap_or_else(|| "SLEEP NOMINAL".to_string());
                ui.vertical(|ui| {
                    ui_components::stat_card(
                        ui,
                        180.0,
                        &format!("ACTIGRAPHY (TARGET {}M)", snap.sleep_target_min),
                        &sleep_txt,
                        INK_PRIMARY,
                        theme,
                    );
                });
            });
        });
    } else {
        // INPUT MODE: Glove-friendly ISS Food Cache selector + Meal % + Log Meal / Hydration
        let food_count = app.snapshot.food_cache.len();
        let selected_idx = app.selected_food_idx.min(food_count.saturating_sub(1));

        ui.label(
            egui::RichText::new(format!(
                "ISS RATION & HYDRATION LOG  —  {}",
                app.status_banner
            ))
            .size(28.0)
            .strong()
            .color(INK_PRIMARY),
        );
        ui.add_space(6.0);

        // Row 1: 4 Food Cache buttons (>= 140x100)
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
                    format!("{:.0}KCAL", f.kcal),
                )
            })
            .collect();

        ui.columns(4, |cols| {
            for (idx, left, right) in &foods {
                if let Some(col) = cols.get_mut(*idx) {
                    let w = col.available_width();
                    if ui_components::touch_button(
                        col,
                        &ctx,
                        ("food_select", *idx),
                        Vec2::new(w, 108.0),
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
            }
        });

        ui.add_space(10.0);

        // Row 2: 4 Consumption percentage buttons (100%, 50%, 25%, 0%)
        ui.columns(4, |cols| {
            for (i, pct) in [100_u8, 50, 25, 0].iter().enumerate() {
                let w = cols[i].available_width();
                let label = format!("PORTION {}%", pct);
                if ui_components::touch_button(
                    &mut cols[i],
                    &ctx,
                    ("meal_pct_btn", *pct),
                    Vec2::new(w, 108.0),
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

        ui.add_space(10.0);

        // Row 3: Action commit buttons + live nutrition totals card
        ui.columns(3, |cols| {
            let w0 = cols[0].available_width();
            if ui_components::touch_button(
                &mut cols[0],
                &ctx,
                "commit_meal_log",
                Vec2::new(w0, 118.0),
                "ÖĞÜNÜ KAYDET",
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
                Vec2::new(w1, 118.0),
                "+250 mL KAYDET",
                "SIVI",
                false,
                CardTheme::INPUT,
            )
            .clicked()
            {
                app.log_hydration_pouch();
            }

            ui_components::stat_card(
                &mut cols[2],
                118.0,
                "GÜNLÜK TOPLAM",
                &format!(
                    "{:.0} KCAL | {:.0} ML",
                    app.snapshot.daily_nutrition.kcal_total,
                    app.snapshot.daily_nutrition.fluid_ml_total
                ),
                INK_PRIMARY,
                CardTheme::VIEW,
            );
        });
    }
}

fn render_hub_nav_detail(ui: &mut egui::Ui, app: &mut BiomonApp, theme: CardTheme, is_alert: bool) {
    let ctx = ui.ctx().clone();
    let val_col = if is_alert { OUTLINE_ALERT } else { INK_PRIMARY };

    // Row 1: Range, Bearing, Phase/Sol, G-Load/IMU
    ui.columns(4, |cols| {
        ui_components::stat_card(
            &mut cols[0],
            118.0,
            "ÜS UZAKLIĞI",
            &format!("{} M", app.snapshot.hub_distance_m),
            val_col,
            theme,
        );
        ui_components::stat_card(
            &mut cols[1],
            118.0,
            "AÇI / YÖN",
            &format!(
                "{:03.0}° {}",
                app.snapshot.heading_deg,
                app.snapshot.cardinal_heading()
            ),
            INK_PRIMARY,
            theme,
        );
        ui_components::stat_card(
            &mut cols[2],
            118.0,
            "PHASE / MISSION SOL",
            &format!(
                "{} / SOL {:02}",
                app.snapshot.active_phase, app.snapshot.mission_day
            ),
            INK_PRIMARY,
            theme,
        );
        ui_components::stat_card(
            &mut cols[3],
            118.0,
            "G-LOAD / IMU ACT",
            &format!(
                "{:.2} G / {:.2}",
                app.snapshot.g_load, app.snapshot.imu_activity_idx
            ),
            INK_PRIMARY,
            theme,
        );
    });

    ui.add_space(12.0);

    if app.mode == AppMode::ViewMode {
        ui.columns(2, |cols| {
            cols[0].horizontal(|ui| {
                ui_components::compass_dial(
                    ui,
                    220.0,
                    app.snapshot.heading_deg,
                    app.snapshot.target_bearing_deg,
                    if is_alert {
                        OUTLINE_ALERT
                    } else {
                        OUTLINE_INPUT
                    },
                );
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    ui_components::stat_card(
                        ui,
                        104.0,
                        "EKİP PROFİLİ",
                        &format!(
                            "{} ({:.0}KG {:.0}CM)",
                            app.snapshot.callsign, app.snapshot.weight_kg, app.snapshot.height_cm
                        ),
                        INK_PRIMARY,
                        theme,
                    );
                    ui.add_space(10.0);
                    ui_components::stat_card(
                        ui,
                        104.0,
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
                104.0,
                "EARTH–MARS EPHEMERIS",
                &format!(
                    "{:.2} AU  |  {:04.1} MIN LIGHT",
                    app.snapshot.earth_mars_au, app.snapshot.one_way_light_min
                ),
                INK_PRIMARY,
                theme,
            );
            cols[1].add_space(10.0);
            ui_components::stat_card(
                &mut cols[1],
                104.0,
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
        // INPUT MODE: Mission Phase Switcher + Heading & Range simulation controls (>= 140x100)
        ui.label(
            egui::RichText::new("GLOVE FLIGHT PHASE & NAVIGATION CONTROLS")
                .size(28.0)
                .strong()
                .color(INK_PRIMARY),
        );
        ui.add_space(6.0);

        ui.columns(4, |cols| {
            for (i, phase) in ["LAUNCH", "ORBIT", "EVA", "REENTRY"].iter().enumerate() {
                let w = cols[i].available_width();
                let active = app.snapshot.active_phase.eq_ignore_ascii_case(phase);
                if ui_components::touch_button(
                    &mut cols[i],
                    &ctx,
                    ("nav_phase_btn", *phase),
                    Vec2::new(w, 112.0),
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

        ui.add_space(10.0);

        ui.columns(4, |cols| {
            let w0 = cols[0].available_width();
            if ui_components::touch_button(
                &mut cols[0],
                &ctx,
                "nav_hdg_left",
                Vec2::new(w0, 112.0),
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
                Vec2::new(w1, 112.0),
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
                Vec2::new(w2, 112.0),
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
                Vec2::new(w3, 112.0),
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

fn render_comms_detail(ui: &mut egui::Ui, app: &mut BiomonApp, theme: CardTheme, is_alert: bool) {
    let ctx = ui.ctx().clone();
    let val_col = if is_alert { OUTLINE_ALERT } else { INK_PRIMARY };

    // Row 1: Link State, Target Hub TCP, Unsynced Queue Breakdown, Light Delay
    ui.columns(4, |cols| {
        ui_components::stat_card(
            &mut cols[0],
            118.0,
            "YEREL AĞ BAĞLANTISI",
            app.snapshot.comms_state.label(),
            val_col,
            theme,
        );
        ui_components::stat_card(
            &mut cols[1],
            118.0,
            "ÜS ADRESİ",
            app.lan_client.target_hub_addr(),
            INK_PRIMARY,
            theme,
        );
        ui_components::stat_card(
            &mut cols[2],
            118.0,
            "BEKLEYEN KAYITLAR",
            &format!(
                "{} TOTAL ({} INP)",
                app.snapshot.unsynced.total(),
                app.snapshot.unsynced.crew_inputs
            ),
            INK_PRIMARY,
            theme,
        );
        ui_components::stat_card(
            &mut cols[3],
            118.0,
            "TEK YÖN GECİKME",
            &format!("{:04.1} MIN", app.snapshot.one_way_light_min),
            INK_PRIMARY,
            theme,
        );
    });

    ui.add_space(12.0);

    // Row 2: Queue breakdown + Signal Bars + Recent Crew Input Queue
    ui.columns(2, |cols| {
        cols[0].horizontal(|ui| {
            ui_components::signal_bars(
                ui,
                110.0,
                app.snapshot.comms_state == ConnectionState::Online,
            );
            ui.add_space(14.0);
            ui.vertical(|ui| {
                ui_components::stat_card(
                    ui,
                    118.0,
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
            118.0,
            "LATEST LOGGED CREW INPUT",
            &latest_input_str,
            INK_PRIMARY,
            theme,
        );
    });

    ui.add_space(14.0);

    // Row 3: Interactive Sync / Link Test buttons (available in both modes, >= 140x100)
    ui.columns(3, |cols| {
        let w0 = cols[0].available_width();
        if ui_components::touch_button(
            &mut cols[0],
            &ctx,
            "comms_flush_btn",
            Vec2::new(w0, 118.0),
            "KUYRUĞU EŞİTLE",
            &format!("{} REC", app.snapshot.unsynced.total()),
            false,
            CardTheme::INPUT,
        )
        .clicked()
        {
            app.sync_queued_packets();
        }

        let w1 = cols[1].available_width();
        let toggle_lbl = match app.snapshot.comms_state {
            ConnectionState::Online => "KESİNTİYİ DENE",
            ConnectionState::Offline => "BAĞLANTIYI AÇ",
        };
        if ui_components::touch_button(
            &mut cols[1],
            &ctx,
            "comms_toggle_link",
            Vec2::new(w1, 118.0),
            toggle_lbl,
            "",
            app.snapshot.comms_state == ConnectionState::Offline,
            CardTheme::INPUT,
        )
        .clicked()
        {
            let new_state = app.lan_client.toggle_connection_state();
            app.snapshot.comms_state = new_state;
            app.snapshot.recompute_derived();
            app.status_banner = format!("LAN STATE: {}", new_state.label());
        }

        ui_components::stat_card(
            &mut cols[2],
            118.0,
            "İLETİLEN PAKETLER",
            &format!("{} PKTS", app.lan_client.frames_sent()),
            INK_PRIMARY,
            theme,
        );
    });
}

fn render_hazard_detail(ui: &mut egui::Ui, app: &mut BiomonApp, theme: CardTheme, is_alert: bool) {
    let ctx = ui.ctx().clone();
    let val_col = if is_alert { OUTLINE_ALERT } else { INK_PRIMARY };

    // Row 1: Environmental & SANS telemetry cards
    ui.columns(4, |cols| {
        ui_components::stat_card(
            &mut cols[0],
            114.0,
            "RADYASYON DOZ HIZI",
            &format!("{:.2} USV/H", app.snapshot.radiation_usv_hr),
            val_col,
            theme,
        );
        ui_components::stat_card(
            &mut cols[1],
            114.0,
            "KABİN CO₂ / O₂",
            &format!(
                "{:.0} PPM / {:.1}%",
                app.snapshot.cabin_co2_ppm,
                app.snapshot.cabin_o2_pct.unwrap_or(20.9)
            ),
            val_col,
            theme,
        );
        ui_components::stat_card(
            &mut cols[2],
            114.0,
            "KABİN BASINÇ / NEM",
            &format!(
                "{:.1} KPA / {:.0}%",
                app.snapshot.cabin_pressure_kpa.unwrap_or(101.3),
                app.snapshot.cabin_humidity_pct.unwrap_or(44.0)
            ),
            INK_PRIMARY,
            theme,
        );
        let sans_str = app
            .snapshot
            .sans_state
            .as_ref()
            .map(|s| format!("{} ({:.0})", s.sans_risk, s.sans_score))
            .unwrap_or_else(|| "LOW (24)".to_string());
        ui_components::stat_card(
            &mut cols[3],
            114.0,
            "SANS RİSK DÜZEYİ",
            &sans_str,
            INK_PRIMARY,
            theme,
        );
    });

    ui.add_space(10.0);

    // Row 2: Selector buttons for the 5 deterministic Offline Protocols
    let proto_count = app.snapshot.protocols.len();
    let sel_idx = app.selected_protocol_idx.min(proto_count.saturating_sub(1));

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
                    Vec2::new(w, 102.0),
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

    ui.add_space(10.0);

    // Row 3: Selected Offline Protocol Directive + Countermeasure Actions
    if let Some(proto) = app.snapshot.protocols.get(sel_idx).cloned() {
        ui.columns(3, |cols| {
            ui_components::stat_card(
                &mut cols[0],
                122.0,
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
                Vec2::new(w1, 122.0),
                "O2 PREBREATHE +10M",
                &format!("R={:.2}", app.snapshot.blood_n2_idx.unwrap_or(1.08)),
                false,
                CardTheme::INPUT,
            )
            .clicked()
            {
                app.step_prebreathe_washout(600.0);
            }

            let unack = app
                .snapshot
                .local_alerts
                .iter()
                .filter(|a| a.acknowledged == 0)
                .count();
            let w2 = cols[2].available_width();
            if ui_components::touch_button(
                &mut cols[2],
                &ctx,
                "ack_local_alerts_btn",
                Vec2::new(w2, 122.0),
                "UYARILARI ONAYLA",
                &format!("{} OPEN", unack),
                unack > 0,
                CardTheme::INPUT,
            )
            .clicked()
            {
                app.acknowledge_alerts();
            }
        });

        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(format!("DIRECTIVE: {}", proto.immediate_action))
                .size(28.0)
                .strong()
                .color(if is_alert { OUTLINE_ALERT } else { INK_PRIMARY }),
        );
    }
}
