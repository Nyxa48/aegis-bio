use crate::app::{
    CrewSummarySnapshot, HubApp, HubModule, HubScreen, HubUiSnapshot, MissionPhase, StressScenario,
};
use crate::calc;
use crate::icons;
use crate::ui_components::{self, CardTheme, StatusLevel, palette};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, RichText, Sense, Stroke,
    StrokeKind, Vec2, epaint::Shadow,
};

// Re-export mandated palette constants
pub use palette::{
    BG_ACTIVE, BG_HOVER, BG_PRIMARY, INK_PRIMARY, OUTLINE_ALERT, OUTLINE_INPUT, OUTLINE_VIEW,
};

const BORDER_DESKTOP: f32 = 3.0;

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
    visuals.window_stroke = Stroke::new(BORDER_DESKTOP, INK_PRIMARY);

    visuals.widgets.noninteractive.bg_fill = BG_PRIMARY;
    visuals.widgets.noninteractive.weak_bg_fill = BG_PRIMARY;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(BORDER_DESKTOP, OUTLINE_VIEW);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.8, INK_PRIMARY);

    visuals.widgets.inactive.bg_fill = BG_PRIMARY;
    visuals.widgets.inactive.weak_bg_fill = BG_PRIMARY;
    visuals.widgets.inactive.bg_stroke = Stroke::new(BORDER_DESKTOP, OUTLINE_VIEW);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.8, INK_PRIMARY);

    visuals.widgets.hovered.bg_fill = BG_HOVER;
    visuals.widgets.hovered.weak_bg_fill = BG_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(BORDER_DESKTOP, OUTLINE_INPUT);
    visuals.widgets.hovered.fg_stroke = Stroke::new(2.0, INK_PRIMARY);

    visuals.widgets.active.bg_fill = BG_ACTIVE;
    visuals.widgets.active.weak_bg_fill = BG_ACTIVE;
    visuals.widgets.active.bg_stroke = Stroke::new(BORDER_DESKTOP, OUTLINE_INPUT);
    visuals.widgets.active.fg_stroke = Stroke::new(2.0, INK_PRIMARY);

    ctx.set_visuals(visuals);
}

pub fn render(ui: &mut egui::Ui, app: &mut HubApp) {
    ui.ctx().request_repaint();
    let time_sec = ui.input(|i| i.time);

    // 1. TOP STATUS & TAB NAVIGATION PANEL (egui::Panel::top)
    let top_border = if app.snapshot.alerts.health_danger
        || app.snapshot.alerts.hazard_danger
        || app.snapshot.alerts.hub_danger
    {
        OUTLINE_ALERT
    } else {
        OUTLINE_VIEW
    };

    egui::Panel::top("hub_status_header")
        .exact_size(114.0)
        .frame(
            egui::Frame::NONE
                .fill(BG_PRIMARY)
                .stroke(Stroke::new(BORDER_DESKTOP, top_border))
                .inner_margin(egui::Margin::symmetric(16, 10)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("AEGIS-BIO // GÖREV KONTROL")
                        .size(24.0)
                        .strong()
                        .color(INK_PRIMARY),
                );

                ui.add_space(10.0);
                let sys_level = if app.snapshot.alerts.health_danger
                    || app.snapshot.alerts.hazard_danger
                    || app.snapshot.alerts.hub_danger
                {
                    StatusLevel::Critical
                } else {
                    StatusLevel::Nominal
                };
                ui_components::status_badge(
                    ui,
                    sys_level,
                    if sys_level == StatusLevel::Critical {
                        "EKİP UYARISI"
                    } else {
                        "OLAĞAN"
                    },
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    draw_status_badge(ui, "DB: READY", OUTLINE_VIEW, false);
                    ui.add_space(8.0);
                    let lan_label = if app.snapshot.lan_ready {
                        "LAN :7878 READY"
                    } else {
                        "LAN :7878 OFFLINE"
                    };
                    draw_lan_badge(ui, lan_label, app.snapshot.lan_ready);
                    ui.add_space(8.0);
                    draw_status_badge(
                        ui,
                        &format!(
                            "MARS {:.2} AU | OWLT {:.2}M",
                            app.snapshot.distance_au, app.snapshot.one_way_light_min
                        ),
                        OUTLINE_VIEW,
                        true,
                    );
                });
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                let avail_w = ui.available_width();
                let btn_w = ((avail_w - 36.0) / 5.0).clamp(140.0, 240.0);
                let overview_active = app.screen == HubScreen::MainMenu;
                if nav_tab_button(ui, "ANA MENÜ", overview_active, OUTLINE_VIEW, btn_w)
                    .clicked()
                {
                    app.screen = HubScreen::MainMenu;
                }
                for module in [
                    HubModule::CrewVitalsMonitor,
                    HubModule::PhaseAndStressControl,
                    HubModule::MedicalAndIncidentLog,
                    HubModule::FoodCatalogAndSync,
                ] {
                    let active = app.screen == HubScreen::Module(module);
                    let border = module_border_color(module, &app.snapshot);
                    if nav_tab_button(ui, module.title(), active, border, btn_w).clicked() {
                        app.screen = HubScreen::Module(module);
                    }
                }
            });
        });

    // 2. BOTTOM STATUS BAR (egui::Panel::bottom)
    let owlt_total_sec = (app.snapshot.one_way_light_min * 60.0).round().max(1.0) as u64;
    let elapsed_cycle = (time_sec.max(0.0) as u64) % owlt_total_sec;
    let rem_sec = owlt_total_sec.saturating_sub(elapsed_cycle);
    let rem_mm = rem_sec / 60;
    let rem_ss = rem_sec % 60;

    egui::Panel::bottom("hub_status_footer")
        .exact_size(38.0)
        .frame(
            egui::Frame::NONE
                .fill(BG_PRIMARY)
                .stroke(Stroke::new(BORDER_DESKTOP, OUTLINE_VIEW))
                .inner_margin(egui::Margin::symmetric(16, 6)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (comp_rect, _) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
                let active_heading = app
                    .snapshot
                    .crew_summaries
                    .get(app.selected_crew_idx)
                    .map(|c| c.heading_deg)
                    .unwrap_or(38.0);
                icons::draw_compass_icon(
                    ui.painter(),
                    comp_rect.center(),
                    9.0,
                    INK_PRIMARY,
                    active_heading,
                );
                ui.label(
                    RichText::new(format!("DB: {}", app.snapshot.db_path))
                        .size(15.0)
                        .family(FontFamily::Monospace)
                        .strong()
                        .color(INK_PRIMARY),
                );
                ui.add_space(12.0);
                ui.label(
                    RichText::new(format!(
                        "BIND: {} ({}) | CLIENTS: {} | PKTS RX: {}",
                        app.lan_server.bind_addr(),
                        if app.snapshot.lan_ready {
                            "ONLINE"
                        } else {
                            "OFFLINE"
                        },
                        app.snapshot.connected_clients,
                        app.snapshot.packets_received
                    ))
                    .size(15.0)
                    .family(FontFamily::Monospace)
                    .strong()
                    .color(INK_PRIMARY),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "OWLT COUNTDOWN: {:02}:{:02} (DELAY {:.2} MIN)",
                            rem_mm, rem_ss, app.snapshot.one_way_light_min
                        ))
                        .size(15.0)
                        .family(FontFamily::Monospace)
                        .strong()
                        .color(INK_PRIMARY),
                    );
                });
            });
        });

    // 3. LEFT SIDEBAR: CREW ROSTER, PHASE TOUCH BUTTONS & STRESS GAIN SLIDER (egui::Panel::left)
    egui::Panel::left("hub_crew_sidebar")
        .exact_size(285.0)
        .frame(
            egui::Frame::NONE
                .fill(BG_PRIMARY)
                .stroke(Stroke::new(BORDER_DESKTOP, OUTLINE_VIEW))
                .inner_margin(egui::Margin::symmetric(14, 12)),
        )
        .show(ui, |ui| {
            ui.label(
                RichText::new("EKİP LİSTESİ")
                    .size(22.0)
                    .strong()
                    .color(INK_PRIMARY),
            );
            ui.add_space(6.0);

            let mut clicked_crew: Option<usize> = None;
            for (idx, crew) in app.snapshot.crew_summaries.iter().enumerate() {
                let selected = app.selected_crew_idx == idx;
                let alert = crew.alerts.health_danger
                    || crew.alerts.hazard_danger
                    || crew.alerts.hub_danger;
                let border = if alert {
                    OUTLINE_ALERT
                } else if selected {
                    OUTLINE_INPUT
                } else {
                    OUTLINE_VIEW
                };
                let fill = if selected { BG_ACTIVE } else { BG_PRIMARY };
                let bpm_u16 = crew.hr_bpm.round().clamp(30.0, 240.0) as u16;

                let resp = egui::Frame::NONE
                    .fill(fill)
                    .stroke(Stroke::new(BORDER_DESKTOP, border))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(egui::Margin::symmetric(10, 7))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            let (icon_rect, _) =
                                ui.allocate_exact_size(Vec2::new(22.0, 22.0), Sense::hover());
                            icons::draw_heart_icon(
                                ui.painter(),
                                icon_rect.center(),
                                10.0,
                                if alert { OUTLINE_ALERT } else { INK_PRIMARY },
                                bpm_u16,
                                time_sec,
                                true,
                            );
                            ui.label(
                                RichText::new(&crew.member.astronaut_id)
                                    .size(16.0)
                                    .strong()
                                    .color(INK_PRIMARY),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(format!("{:.0} BPM", crew.hr_bpm))
                                            .size(16.0)
                                            .family(FontFamily::Monospace)
                                            .strong()
                                            .color(if alert { OUTLINE_ALERT } else { INK_PRIMARY }),
                                    );
                                },
                            );
                        });
                    })
                    .response
                    .interact(Sense::click());

                if resp.clicked() {
                    clicked_crew = Some(idx);
                }
                ui.add_space(5.0);
            }
            if let Some(idx) = clicked_crew {
                app.select_crew(idx);
            }

            ui.add_space(8.0);
            ui.label(
                RichText::new("GÖREV FAZI")
                    .size(22.0)
                    .strong()
                    .color(INK_PRIMARY),
            );
            ui.add_space(5.0);

            for (i, phase) in [
                MissionPhase::Launch,
                MissionPhase::Orbit,
                MissionPhase::Eva,
                MissionPhase::Reentry,
            ]
            .iter()
            .enumerate()
            {
                let selected = app.mission_phase == *phase;
                let right_tag = if selected { "ACTIVE" } else { "" };
                if ui_components::touch_button(
                    ui,
                    ("hub_phase_btn", i),
                    38.0,
                    phase.label(),
                    right_tag,
                    selected,
                    CardTheme::INPUT,
                )
                .clicked()
                {
                    app.set_mission_phase(*phase);
                }
                ui.add_space(4.0);
            }

            ui.add_space(6.0);
            let gain_u16 = app.stress_gain.round().clamp(25.0, 175.0) as u16;
            if let Some(next_val) = ui_components::progress_slider(
                ui,
                46.0,
                "STRES DÜZEYİ",
                gain_u16,
                25,
                175,
                CardTheme::INPUT,
            ) {
                app.stress_gain = next_val as f32;
            }
        });

    // 4. CENTRAL PANEL (egui::CentralPanel + ScrollArea::vertical)
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(palette::BG_OUTER))
        .show(ui, |ui| match app.screen {
            HubScreen::MainMenu => render_main_menu(ui, app, time_sec),
            HubScreen::Module(module) => {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Frame::NONE
                            .inner_margin(egui::Margin::symmetric(18, 14))
                            .show(ui, |ui| match module {
                                HubModule::CrewVitalsMonitor => {
                                    render_crew_vitals_module(ui, app, time_sec)
                                }
                                HubModule::PhaseAndStressControl => {
                                    render_phase_and_stress_module(ui, app, time_sec)
                                }
                                HubModule::MedicalAndIncidentLog => {
                                    render_medical_and_incident_module(ui, app)
                                }
                                HubModule::FoodCatalogAndSync => {
                                    render_food_and_sync_module(ui, app, time_sec)
                                }
                            });
                    });
            }
        });
}

fn module_border_color(module: HubModule, snap: &HubUiSnapshot) -> Color32 {
    match module {
        HubModule::CrewVitalsMonitor => {
            if snap.alerts.health_danger || snap.alerts.hub_danger {
                OUTLINE_ALERT
            } else {
                OUTLINE_VIEW
            }
        }
        HubModule::PhaseAndStressControl => {
            if snap.alerts.health_danger || snap.alerts.hazard_danger || snap.alerts.hub_danger {
                OUTLINE_ALERT
            } else {
                OUTLINE_INPUT
            }
        }
        HubModule::MedicalAndIncidentLog => {
            if snap.alerts.hazard_danger || snap.unacked_incidents > 0 {
                OUTLINE_ALERT
            } else {
                OUTLINE_VIEW
            }
        }
        HubModule::FoodCatalogAndSync => {
            if snap.alerts.comms_danger {
                OUTLINE_ALERT
            } else {
                OUTLINE_VIEW
            }
        }
    }
}

fn draw_lan_badge(ui: &mut egui::Ui, text: &str, online: bool) {
    let border_color = if online { OUTLINE_VIEW } else { OUTLINE_ALERT };

    egui::Frame::NONE
        .fill(BG_PRIMARY)
        .stroke(Stroke::new(BORDER_DESKTOP, border_color))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(10, 5))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui_components::signal_bars(ui, 18.0, online);
                ui.label(RichText::new(text).size(15.0).strong().color(INK_PRIMARY));
            });
        });
}

fn draw_status_badge(ui: &mut egui::Ui, text: &str, border_color: Color32, mono: bool) {
    egui::Frame::NONE
        .fill(BG_PRIMARY)
        .stroke(Stroke::new(BORDER_DESKTOP, border_color))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(10, 5))
        .show(ui, |ui| {
            let mut rt = RichText::new(text).size(15.0).strong().color(INK_PRIMARY);
            if mono {
                rt = rt.family(FontFamily::Monospace);
            }
            ui.label(rt);
        });
}

fn nav_tab_button(
    ui: &mut egui::Ui,
    label: &str,
    active: bool,
    border_color: Color32,
    min_w: f32,
) -> egui::Response {
    let btn = egui::Button::new(RichText::new(label).size(15.0).strong().color(INK_PRIMARY))
        .min_size(Vec2::new(min_w, 34.0))
        .fill(if active { BG_ACTIVE } else { BG_PRIMARY })
        .stroke(Stroke::new(BORDER_DESKTOP, border_color))
        .corner_radius(CornerRadius::same(6));
    ui.add(btn)
}

fn render_main_menu(ui: &mut egui::Ui, app: &mut HubApp, time_sec: f64) {
    let outer = ui.max_rect().shrink(16.0);
    let active_crew = app.snapshot.crew_summaries.get(app.selected_crew_idx);

    let kpi_height = 92.0;
    let kpi_rect = Rect::from_min_size(outer.min, Vec2::new(outer.width(), kpi_height));
    ui.scope_builder(egui::UiBuilder::new().max_rect(kpi_rect), |ui| {
        ui.columns(4, |c| {
            let hr_val = active_crew
                .map(|cr| format!("{:.0} BPM", cr.hr_bpm))
                .unwrap_or_else(|| "88 BPM".to_string());
            let (hr_lo, hr_hi) = calc::phase_hr_corridor(app.mission_phase.label());
            let hr_theme = if app.snapshot.alerts.health_danger {
                CardTheme::ALERT
            } else {
                CardTheme::HEALTH
            };
            ui_components::stat_card(
                &mut c[0],
                kpi_height,
                "NABIZ",
                &hr_val,
                if app.snapshot.alerts.health_danger {
                    OUTLINE_ALERT
                } else {
                    INK_PRIMARY
                },
                &format!("{:.0}–{:.0} BPM", hr_lo, hr_hi),
                hr_theme,
            );

            ui_components::stat_card(
                &mut c[1],
                kpi_height,
                "MARS UZAKLIĞI",
                &format!("{:.2} AU", app.snapshot.distance_au),
                INK_PRIMARY,
                &format!("OWLT {:.2}M", app.snapshot.one_way_light_min),
                CardTheme::HUB,
            );

            ui_components::stat_card(
                &mut c[2],
                kpi_height,
                "YEREL AĞ",
                if app.snapshot.lan_ready {
                    "ONLINE"
                } else {
                    "OFFLINE"
                },
                INK_PRIMARY,
                &format!(":7878 | {} PKTS", app.snapshot.packets_received),
                CardTheme::COMMS,
            );

            let rad_val = active_crew
                .map(|cr| format!("{:.2} uSv/h", cr.radiation_usv_hr))
                .unwrap_or_else(|| "0.42 uSv/h".to_string());
            let haz_theme = if app.snapshot.alerts.hazard_danger {
                CardTheme::ALERT
            } else {
                CardTheme::HAZARD
            };
            ui_components::stat_card(
                &mut c[3],
                kpi_height,
                "RADYASYON",
                &rad_val,
                if app.snapshot.alerts.hazard_danger {
                    OUTLINE_ALERT
                } else {
                    INK_PRIMARY
                },
                "LIMIT < 50.0",
                haz_theme,
            );
        });
    });

    let grid_top = kpi_rect.bottom() + 14.0;
    let rect = Rect::from_min_max(Pos2::new(outer.left(), grid_top), outer.max);
    let gap = 14.0;
    let half_gap = gap * 0.5;
    let mid_x = rect.center().x;
    let mid_y = rect.center().y;

    let tl_rect = Rect::from_min_max(rect.min, Pos2::new(mid_x - half_gap, mid_y - half_gap));
    let tr_rect = Rect::from_min_max(
        Pos2::new(mid_x + half_gap, rect.min.y),
        Pos2::new(rect.max.x, mid_y - half_gap),
    );
    let bl_rect = Rect::from_min_max(
        Pos2::new(rect.min.x, mid_y + half_gap),
        Pos2::new(mid_x - half_gap, rect.max.y),
    );
    let br_rect = Rect::from_min_max(Pos2::new(mid_x + half_gap, mid_y + half_gap), rect.max);

    let hr_summary = active_crew
        .map(|c| {
            format!(
                "{} | {:.0} BPM | {:.0} MMHG | O2 {:.0}%",
                c.member.astronaut_id, c.hr_bpm, c.sbp_mmhg, c.spo2_pct
            )
        })
        .unwrap_or_else(|| "AST-01 | 88 BPM | 118 MMHG | O2 98%".to_string());

    let phase_summary = format!(
        "PHASE: {} | GAIN: {:.0}% | SANS: {}",
        app.mission_phase.label(),
        app.stress_gain,
        active_crew.map(|c| c.sans_risk.as_str()).unwrap_or("LOW")
    );
    let incident_summary = format!(
        "UNACKED INCIDENTS: {} | LAB REPORTS: {}",
        app.snapshot.unacked_incidents,
        app.snapshot.lab_reports.len()
    );
    let food_summary = format!(
        "ISS CATALOG: {} | INPUT LOGS: {} | EPHEM: {}",
        app.snapshot.food_catalog.len(),
        app.snapshot.crew_inputs.len(),
        app.snapshot.mission_date
    );

    let modules = [
        (tl_rect, HubModule::CrewVitalsMonitor, hr_summary),
        (tr_rect, HubModule::PhaseAndStressControl, phase_summary),
        (bl_rect, HubModule::MedicalAndIncidentLog, incident_summary),
        (br_rect, HubModule::FoodCatalogAndSync, food_summary),
    ];

    let pointer_down = ui.input(|i| i.pointer.primary_down());

    for (mod_rect, module, subtitle) in modules {
        let border_color = module_border_color(module, &app.snapshot);
        let response = ui.allocate_rect(mod_rect, Sense::click());
        let fill = if response.hovered() && pointer_down {
            BG_ACTIVE
        } else if response.hovered() {
            BG_HOVER
        } else {
            BG_PRIMARY
        };

        let painter = ui.painter();
        painter.rect(
            mod_rect,
            CornerRadius::same(10),
            fill,
            Stroke::new(BORDER_DESKTOP, border_color),
            StrokeKind::Inside,
        );

        let icon_center = Pos2::new(mod_rect.center().x, mod_rect.center().y - 38.0);
        draw_module_icon(
            painter,
            module,
            icon_center,
            28.0,
            time_sec,
            &app.snapshot,
            app.selected_crew_idx,
        );

        painter.text(
            Pos2::new(mod_rect.center().x, mod_rect.center().y + 12.0),
            Align2::CENTER_CENTER,
            module.title(),
            FontId::new(26.0, FontFamily::Proportional),
            INK_PRIMARY,
        );

        painter.text(
            Pos2::new(mod_rect.center().x, mod_rect.center().y + 46.0),
            Align2::CENTER_CENTER,
            subtitle,
            FontId::new(16.0, FontFamily::Monospace),
            INK_PRIMARY,
        );

        if response.clicked() {
            app.screen = HubScreen::Module(module);
        }
    }
}

fn draw_module_icon(
    painter: &egui::Painter,
    module: HubModule,
    center: Pos2,
    radius: f32,
    time_sec: f64,
    snap: &HubUiSnapshot,
    selected_idx: usize,
) {
    match module {
        HubModule::CrewVitalsMonitor => {
            let bpm = snap
                .crew_summaries
                .get(selected_idx)
                .map(|c| c.hr_bpm.round().clamp(30.0, 240.0) as u16)
                .unwrap_or(88);
            let color = if snap.alerts.health_danger {
                OUTLINE_ALERT
            } else {
                INK_PRIMARY
            };
            icons::draw_heart_icon(painter, center, radius, color, bpm, time_sec, true);
        }
        HubModule::PhaseAndStressControl => {
            icons::draw_shield_icon(painter, center, radius, INK_PRIMARY);
        }
        HubModule::MedicalAndIncidentLog => {
            let color = if snap.alerts.hazard_danger || snap.unacked_incidents > 0 {
                OUTLINE_ALERT
            } else {
                INK_PRIMARY
            };
            icons::draw_hazard_icon(painter, center, radius, color);
        }
        HubModule::FoodCatalogAndSync => {
            icons::draw_nutrition_crate_icon(painter, center, radius, INK_PRIMARY);
        }
    }
}

fn section_frame(border: Color32) -> egui::Frame {
    egui::Frame::NONE
        .fill(BG_PRIMARY)
        .stroke(Stroke::new(BORDER_DESKTOP, border))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(16, 12))
}

fn render_module_header_bar(ui: &mut egui::Ui, app: &mut HubApp, module: HubModule, time_sec: f64) {
    let border = module_border_color(module, &app.snapshot);
    section_frame(border).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::hover());
            draw_module_icon(
                ui.painter(),
                module,
                icon_rect.center(),
                15.0,
                time_sec,
                &app.snapshot,
                app.selected_crew_idx,
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(module.title())
                    .size(26.0)
                    .strong()
                    .color(INK_PRIMARY),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let back_btn = egui::Button::new(
                    RichText::new("ANA MENÜYE DÖN")
                        .size(16.0)
                        .strong()
                        .color(INK_PRIMARY),
                )
                .min_size(Vec2::new(220.0, 36.0))
                .fill(BG_PRIMARY)
                .stroke(Stroke::new(BORDER_DESKTOP, OUTLINE_INPUT))
                .corner_radius(CornerRadius::same(6));
                if ui.add(back_btn).clicked() {
                    app.screen = HubScreen::MainMenu;
                }
            });
        });
    });
}

fn suit_zones_for_crew(crew: &CrewSummarySnapshot) -> [Color32; 5] {
    let helmet = if crew.spo2_pct < 92.0 || crew.tissue_o2_pct.is_some_and(|t| t < 85.0) {
        OUTLINE_ALERT
    } else {
        OUTLINE_VIEW
    };
    let torso = if crew.alerts.health_danger {
        OUTLINE_ALERT
    } else {
        OUTLINE_VIEW
    };
    let scrubber = if crew.cabin_co2_ppm > 3000.0 {
        OUTLINE_ALERT
    } else {
        OUTLINE_VIEW
    };
    let gloves = if crew
        .suit_pressure_kpa
        .is_some_and(|p| p < 25.0 && crew.member.active_phase_code == "EVA")
    {
        OUTLINE_ALERT
    } else {
        INK_PRIMARY
    };
    let shield = if crew.radiation_usv_hr > 50.0 {
        OUTLINE_ALERT
    } else {
        OUTLINE_VIEW
    };
    [helmet, torso, scrubber, gloves, shield]
}

// ============================================================================
// MODULE 1: CREW VITALS MONITOR
// ============================================================================
fn render_crew_vitals_module(ui: &mut egui::Ui, app: &mut HubApp, time_sec: f64) {
    render_module_header_bar(ui, app, HubModule::CrewVitalsMonitor, time_sec);
    ui.add_space(12.0);

    let Some(crew) = app
        .snapshot
        .crew_summaries
        .get(app.selected_crew_idx)
        .cloned()
    else {
        return;
    };

    let header_border =
        if crew.alerts.health_danger || crew.alerts.hazard_danger || crew.alerts.hub_danger {
            OUTLINE_ALERT
        } else {
            OUTLINE_VIEW
        };

    // Deep-dive selected crew bio-profile + suit schematic + compass + HR corridor
    section_frame(header_border).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} // {}",
                        crew.member.astronaut_id, crew.member.full_name
                    ))
                    .size(24.0)
                    .strong()
                    .color(INK_PRIMARY),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!(
                        "ROLE: {}  |  PHASE: {}  |  LAN: {}  |  HEARTBEAT: {}",
                        crew.member.role,
                        crew.member.active_phase_code,
                        crew.member.lan_status,
                        crew.member.last_heartbeat_utc
                    ))
                    .size(16.0)
                    .family(FontFamily::Monospace)
                    .strong()
                    .color(INK_PRIMARY),
                );
                ui.label(
                    RichText::new(format!(
                        "BIO: {:.1} KG  |  {:.0} CM  |  AGE {}  |  SEX {}  |  SLEEP TARGET: {} MIN  |  SEQ #{}",
                        crew.member.weight,
                        crew.member.height,
                        crew.member.age,
                        crew.member.sex,
                        crew.member.sleep_target_min,
                        crew.edge_seq_id
                    ))
                    .size(16.0)
                    .family(FontFamily::Monospace)
                    .color(INK_PRIMARY),
                );
                ui.add_space(8.0);
                let (lo, hi) = calc::phase_hr_corridor(&crew.member.active_phase_code);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "HR PHASE CORRIDOR ({:.0}–{:.0} BPM):",
                            lo, hi
                        ))
                        .size(16.0)
                        .strong()
                        .color(INK_PRIMARY),
                    );
                    ui_components::corridor_bar(
                        ui,
                        320.0,
                        crew.hr_bpm as f32,
                        lo as f32,
                        hi as f32,
                        if crew.hr_bpm < lo || crew.hr_bpm > hi {
                            OUTLINE_ALERT
                        } else {
                            OUTLINE_VIEW
                        },
                    );
                });
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui_components::suit_schematic(ui, 126.0, suit_zones_for_crew(&crew));
                ui.add_space(16.0);
                ui_components::compass_dial(
                    ui,
                    126.0,
                    crew.heading_deg,
                    crew.target_deg,
                    if crew.alerts.hub_danger {
                        OUTLINE_ALERT
                    } else {
                        OUTLINE_INPUT
                    },
                );
            });
        });
    });

    ui.add_space(12.0);

    // Compute metabolic kcal/s and sweat mL/s for selected crew
    let kcal_sec = calc::calc_kcal_per_sec(
        crew.hr_bpm,
        crew.member.weight,
        crew.member.age as i32,
        &crew.member.sex,
    );
    let sweat_sec = calc::calc_sweat_ml_per_sec(kcal_sec * 60.0, crew.skin_temp_c);
    let (hr_lo, hr_hi) = calc::phase_hr_corridor(&crew.member.active_phase_code);
    let card_h = 88.0;

    // Row 1: Primary Cardiorespiratory & Decompression
    ui.columns(4, |c| {
        let hr_bad = crew.hr_bpm < hr_lo || crew.hr_bpm > hr_hi;
        ui_components::stat_card(
            &mut c[0],
            card_h,
            "HEART RATE & RESP",
            &format!("{:.0} BPM / {:.0} BRPM", crew.hr_bpm, crew.resp_brpm),
            if hr_bad { OUTLINE_ALERT } else { INK_PRIMARY },
            &format!("CORRIDOR {:.0}–{:.0} BPM", hr_lo, hr_hi),
            if hr_bad {
                CardTheme::ALERT
            } else {
                CardTheme::HEALTH
            },
        );

        let si_bad = crew.shock_index > 0.90;
        ui_components::stat_card(
            &mut c[1],
            card_h,
            "SYSTOLIC BP & SHOCK IDX",
            &format!("{:.0} MMHG / SI {:.2}", crew.sbp_mmhg, crew.shock_index),
            if si_bad { OUTLINE_ALERT } else { INK_PRIMARY },
            &format!("ORTHO RISK: {}", crew.orthostatic_risk),
            if si_bad {
                CardTheme::ALERT
            } else {
                CardTheme::HEALTH
            },
        );

        let o2_bad = crew.spo2_pct < 92.0 || crew.tissue_o2_pct.is_some_and(|t| t < 85.0);
        ui_components::stat_card(
            &mut c[2],
            card_h,
            "SPO2 & TISSUE O2",
            &format!(
                "{:.1}% / {:.1}%",
                crew.spo2_pct,
                crew.tissue_o2_pct.unwrap_or(93.5)
            ),
            if o2_bad { OUTLINE_ALERT } else { INK_PRIMARY },
            "MIN: SPO2 92% | TIS 85%",
            if o2_bad {
                CardTheme::ALERT
            } else {
                CardTheme::HEALTH
            },
        );

        let n2_val = crew.blood_n2_idx.unwrap_or(1.08);
        let n2_bad = n2_val > 1.50;
        ui_components::stat_card(
            &mut c[3],
            card_h,
            "BLOOD N2 R-VALUE & SANS",
            &format!("R={:.2} / {}", n2_val, crew.sans_risk),
            if n2_bad { OUTLINE_ALERT } else { INK_PRIMARY },
            "HALDANE LIMIT R <= 1.50",
            if n2_bad {
                CardTheme::ALERT
            } else {
                CardTheme::HEALTH
            },
        );
    });

    ui.add_space(10.0);

    // Row 2: Cabin Environment & Radiation
    ui.columns(4, |c| {
        let rad_bad = crew.radiation_usv_hr > 50.0;
        ui_components::stat_card(
            &mut c[0],
            card_h,
            "RADIATION DOSIMETRY",
            &format!("{:.2} uSv/h", crew.radiation_usv_hr),
            if rad_bad { OUTLINE_ALERT } else { INK_PRIMARY },
            "CEILING < 50.0 uSv/h",
            if rad_bad {
                CardTheme::ALERT
            } else {
                CardTheme::HAZARD
            },
        );

        let co2_bad = crew.cabin_co2_ppm > 3000.0 || crew.cabin_o2_pct.is_some_and(|o| o < 19.5);
        ui_components::stat_card(
            &mut c[1],
            card_h,
            "CABIN CO2 & O2",
            &format!(
                "{:.0} PPM / {:.1}%",
                crew.cabin_co2_ppm,
                crew.cabin_o2_pct.unwrap_or(20.9)
            ),
            if co2_bad { OUTLINE_ALERT } else { INK_PRIMARY },
            "CO2 < 3000 | O2 >= 19.5%",
            if co2_bad {
                CardTheme::ALERT
            } else {
                CardTheme::HUB
            },
        );

        ui_components::stat_card(
            &mut c[2],
            card_h,
            "CABIN TEMP & HUMIDITY",
            &format!(
                "{:.1} C / {:.0}% RH",
                crew.cabin_temp_c.unwrap_or(21.5),
                crew.cabin_humidity_pct.unwrap_or(44.0)
            ),
            INK_PRIMARY,
            "SHIRTSLEEVE NOMINAL",
            CardTheme::HUB,
        );

        ui_components::stat_card(
            &mut c[3],
            card_h,
            "CABIN PRESS & MOTION",
            &format!(
                "{:.1} KPA / {:.2} IDX",
                crew.cabin_pressure_kpa.unwrap_or(101.3),
                crew.cabin_motion_idx
            ),
            INK_PRIMARY,
            &format!("DAY {} TELEMETRY", crew.mission_day),
            CardTheme::HUB,
        );
    });

    ui.add_space(10.0);

    // Row 3: Suit, Kinematics & Metabolic Output
    ui.columns(4, |c| {
        let suit_p = crew.suit_pressure_kpa.unwrap_or(29.6);
        let suit_bad = crew.member.active_phase_code == "EVA" && suit_p < 25.0;
        ui_components::stat_card(
            &mut c[0],
            card_h,
            "SUIT PRESS & HUMIDITY",
            &format!(
                "{:.1} KPA / {:.0}%",
                suit_p,
                crew.suit_humidity_pct.unwrap_or(38.0)
            ),
            if suit_bad { OUTLINE_ALERT } else { INK_PRIMARY },
            &format!("EVA ELAPSED: {}S", crew.eva_elapsed_sec),
            if suit_bad {
                CardTheme::ALERT
            } else {
                CardTheme::HEALTH
            },
        );

        ui_components::stat_card(
            &mut c[1],
            card_h,
            "G-LOAD & IMU ACTIVITY",
            &format!("{:.2} G / {:.2} IMU", crew.g_load, crew.imu_activity_idx),
            INK_PRIMARY,
            &format!("SKIN TEMP: {:.1} C", crew.skin_temp_c),
            CardTheme::HEALTH,
        );

        ui_components::stat_card(
            &mut c[2],
            card_h,
            "METABOLIC & SWEAT LOSS",
            &format!("{:.3} KCAL/S | {:.3} ML/S", kcal_sec, sweat_sec),
            INK_PRIMARY,
            &format!(
                "{:.0} KCAL/HR | {:.0} ML/HR",
                kcal_sec * 3600.0,
                sweat_sec * 3600.0
            ),
            CardTheme::HEALTH,
        );

        let range_bad = crew.alerts.hub_danger;
        ui_components::stat_card(
            &mut c[3],
            card_h,
            "HUB RANGE & BEARING",
            &format!(
                "{:.0} M / {:03.0} DEG",
                crew.hub_distance_m, crew.heading_deg
            ),
            if range_bad {
                OUTLINE_ALERT
            } else {
                INK_PRIMARY
            },
            "SAFE RADIUS <= 1500 M",
            if range_bad {
                CardTheme::ALERT
            } else {
                CardTheme::HUB
            },
        );
    });

    ui.add_space(14.0);

    // Full-Roster Side-by-Side Comparison Grid
    section_frame(OUTLINE_VIEW).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label(
            RichText::new("FULL CREW ROSTER TELEMETRY COMPARISON")
                .size(22.0)
                .strong()
                .color(INK_PRIMARY),
        );
        ui.add_space(8.0);

        egui::Grid::new("roster_comparison_grid")
            .striped(true)
            .spacing(Vec2::new(14.0, 8.0))
            .show(ui, |ui| {
                for hdr in [
                    "CREW ID",
                    "CALLSIGN / ROLE",
                    "PHASE",
                    "HR (BPM)",
                    "SBP",
                    "SPO2 / TIS",
                    "N2 R-VAL",
                    "SHOCK / ORTHO",
                    "SANS",
                    "CO2 (PPM)",
                    "RAD (uSv/h)",
                    "SUIT KPA",
                    "RANGE (M)",
                    "STATUS",
                ] {
                    ui.label(RichText::new(hdr).size(15.0).strong().color(INK_PRIMARY));
                }
                ui.end_row();

                for (idx, cr) in app.snapshot.crew_summaries.iter().enumerate() {
                    let row_alert =
                        cr.alerts.health_danger || cr.alerts.hazard_danger || cr.alerts.hub_danger;
                    let col = if row_alert {
                        OUTLINE_ALERT
                    } else {
                        INK_PRIMARY
                    };

                    let select_btn = egui::Button::new(
                        RichText::new(&cr.member.astronaut_id)
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .strong()
                            .color(col),
                    )
                    .fill(if idx == app.selected_crew_idx {
                        BG_ACTIVE
                    } else {
                        BG_PRIMARY
                    })
                    .stroke(Stroke::new(
                        2.0,
                        if row_alert {
                            OUTLINE_ALERT
                        } else {
                            OUTLINE_VIEW
                        },
                    ));
                    if ui.add(select_btn).clicked() {
                        app.selected_crew_idx = idx;
                    }

                    ui.label(
                        RichText::new(format!("{} ({})", cr.member.full_name, cr.member.role))
                            .size(15.0)
                            .color(INK_PRIMARY),
                    );
                    ui.label(
                        RichText::new(&cr.member.active_phase_code)
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .strong()
                            .color(INK_PRIMARY),
                    );
                    ui.label(
                        RichText::new(format!("{:.0}", cr.hr_bpm))
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .strong()
                            .color(col),
                    );
                    ui.label(
                        RichText::new(format!("{:.0}", cr.sbp_mmhg))
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .color(INK_PRIMARY),
                    );
                    ui.label(
                        RichText::new(format!(
                            "{:.1}% / {:.1}%",
                            cr.spo2_pct,
                            cr.tissue_o2_pct.unwrap_or(93.5)
                        ))
                        .size(15.0)
                        .family(FontFamily::Monospace)
                        .color(col),
                    );
                    ui.label(
                        RichText::new(format!("{:.2}", cr.blood_n2_idx.unwrap_or(1.08)))
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .color(col),
                    );
                    ui.label(
                        RichText::new(format!("{:.2} ({})", cr.shock_index, cr.orthostatic_risk))
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .color(col),
                    );
                    ui.label(
                        RichText::new(&cr.sans_risk)
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .color(INK_PRIMARY),
                    );
                    ui.label(
                        RichText::new(format!("{:.0}", cr.cabin_co2_ppm))
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .color(col),
                    );
                    ui.label(
                        RichText::new(format!("{:.2}", cr.radiation_usv_hr))
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .color(col),
                    );
                    ui.label(
                        RichText::new(format!("{:.1}", cr.suit_pressure_kpa.unwrap_or(29.6)))
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .color(col),
                    );
                    ui.label(
                        RichText::new(format!("{:.0}", cr.hub_distance_m))
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .color(col),
                    );
                    ui_components::status_badge(
                        ui,
                        if row_alert {
                            StatusLevel::Critical
                        } else {
                            StatusLevel::Nominal
                        },
                        if row_alert { "ALERT" } else { "OLAĞAN" },
                    );
                    ui.end_row();
                }
            });
    });
}

// ============================================================================
// MODULE 2: PHASE & STRESS CONTROL
// ============================================================================
fn render_phase_and_stress_module(ui: &mut egui::Ui, app: &mut HubApp, time_sec: f64) {
    render_module_header_bar(ui, app, HubModule::PhaseAndStressControl, time_sec);
    ui.add_space(12.0);

    let any_alert = app.snapshot.alerts.health_danger
        || app.snapshot.alerts.hazard_danger
        || app.snapshot.alerts.hub_danger;
    let ctrl_border = if any_alert {
        OUTLINE_ALERT
    } else {
        OUTLINE_INPUT
    };

    section_frame(ctrl_border).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("FLIGHT PHASE & SCOPE CONFIGURATION")
                    .size(22.0)
                    .strong()
                    .color(INK_PRIMARY),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let scope_label = if app.apply_phase_to_all {
                    "TARGET SCOPE: ALL CREW ROSTER (CLICK TO SWITCH)"
                } else {
                    "TARGET SCOPE: SELECTED CREW ONLY (CLICK TO SWITCH)"
                };
                let scope_btn = egui::Button::new(
                    RichText::new(scope_label)
                        .size(15.0)
                        .family(FontFamily::Monospace)
                        .strong()
                        .color(INK_PRIMARY),
                )
                .min_size(Vec2::new(360.0, 36.0))
                .fill(BG_ACTIVE)
                .stroke(Stroke::new(BORDER_DESKTOP, OUTLINE_INPUT));
                if ui.add(scope_btn).clicked() {
                    app.apply_phase_to_all = !app.apply_phase_to_all;
                }
            });
        });

        ui.add_space(10.0);
        ui.columns(4, |c| {
            for (i, phase) in [
                MissionPhase::Launch,
                MissionPhase::Orbit,
                MissionPhase::Eva,
                MissionPhase::Reentry,
            ]
            .iter()
            .enumerate()
            {
                let selected = app.mission_phase == *phase;
                let (lo, hi) = calc::phase_hr_corridor(phase.label());
                let tag = format!("{:.0}-{:.0} BPM", lo, hi);
                if ui_components::touch_button(
                    &mut c[i],
                    ("stress_mod_phase", i),
                    48.0,
                    phase.label(),
                    &tag,
                    selected,
                    CardTheme::INPUT,
                )
                .clicked()
                {
                    app.set_mission_phase(*phase);
                }
            }
        });

        ui.add_space(12.0);
        let gain_u16 = app.stress_gain.round().clamp(25.0, 175.0) as u16;
        if let Some(next_val) = ui_components::progress_slider(
            ui,
            50.0,
            "STRESS GAIN SCALER",
            gain_u16,
            25,
            175,
            CardTheme::INPUT,
        ) {
            app.stress_gain = next_val as f32;
        }
    });

    ui.add_space(12.0);

    // Anomaly Injection Matrix
    section_frame(ctrl_border).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label(
            RichText::new("PHYSIOLOGICAL & ENVIRONMENTAL ANOMALY INJECTION")
                .size(22.0)
                .strong()
                .color(INK_PRIMARY),
        );
        ui.add_space(8.0);

        let scenarios = [
            StressScenario::TachycardiaSpike,
            StressScenario::HypoxiaDrop,
            StressScenario::SolarParticleRadiationSpike,
            StressScenario::CabinCo2ScrubbingFailure,
            StressScenario::SuitDecompressionN2Breach,
            StressScenario::HubRangeExcursion,
        ];

        let active_anom = app
            .snapshot
            .crew_summaries
            .get(app.selected_crew_idx)
            .and_then(|c| c.active_anomaly);

        ui.columns(2, |cols| {
            for (idx, scenario) in scenarios.iter().enumerate() {
                let col_ui = &mut cols[idx % 2];
                let is_active = active_anom == Some(*scenario);
                let right_tag = if is_active { "ACTIVE BREACH" } else { "INJECT" };
                if ui_components::touch_button(
                    col_ui,
                    ("inject_scenario_btn", idx),
                    48.0,
                    scenario.label(),
                    right_tag,
                    is_active,
                    if is_active {
                        CardTheme::ALERT
                    } else {
                        CardTheme::INPUT
                    },
                )
                .clicked()
                {
                    app.inject_stress_scenario(*scenario);
                }
                col_ui.add_space(8.0);
            }
        });

        ui.add_space(6.0);
        ui.columns(2, |cols| {
            if ui_components::touch_button(
                &mut cols[0],
                "reset_nominal_btn",
                50.0,
                "RESET TO NOMINAL DB TELEMETRY",
                "RESTORE SQLITE",
                false,
                CardTheme::HEALTH,
            )
            .clicked()
            {
                app.reset_to_nominal_telemetry();
            }

            if ui_components::touch_button(
                &mut cols[1],
                "log_incident_from_stress_btn",
                50.0,
                "LOG SURGEON INCIDENT FROM ACTIVE TELEMETRY",
                &format!("UNACKED: {}", app.snapshot.unacked_incidents),
                any_alert,
                if any_alert {
                    CardTheme::ALERT
                } else {
                    CardTheme::INPUT
                },
            )
            .clicked()
            {
                app.log_active_alert_incident();
            }
        });
    });

    ui.add_space(12.0);

    // Live Safety Evaluation Readout Table
    section_frame(if any_alert {
        OUTLINE_ALERT
    } else {
        OUTLINE_VIEW
    })
    .show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label(
            RichText::new("REAL-TIME SAFETY EVALUATION & DERIVED RISK MATRIX")
                .size(22.0)
                .strong()
                .color(INK_PRIMARY),
        );
        ui.add_space(8.0);

        egui::Grid::new("safety_eval_matrix_grid")
            .striped(true)
            .spacing(Vec2::new(16.0, 8.0))
            .show(ui, |ui| {
                for hdr in [
                    "CREW",
                    "PHASE",
                    "ACTIVE ANOMALY",
                    "HR / CORRIDOR",
                    "SHOCK IDX",
                    "ORTHO RISK",
                    "SANS RISK",
                    "N2 R-VAL",
                    "HEALTH GATE",
                    "HAZARD GATE",
                    "RANGE GATE",
                ] {
                    ui.label(RichText::new(hdr).size(15.0).strong().color(INK_PRIMARY));
                }
                ui.end_row();

                for crew in &app.snapshot.crew_summaries {
                    let (lo, hi) = calc::phase_hr_corridor(&crew.member.active_phase_code);
                    let row_danger = crew.alerts.health_danger
                        || crew.alerts.hazard_danger
                        || crew.alerts.hub_danger;
                    let text_col = if row_danger {
                        OUTLINE_ALERT
                    } else {
                        INK_PRIMARY
                    };

                    ui.label(
                        RichText::new(&crew.member.astronaut_id)
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .strong()
                            .color(text_col),
                    );
                    ui.label(
                        RichText::new(&crew.member.active_phase_code)
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .color(INK_PRIMARY),
                    );
                    ui.label(
                        RichText::new(
                            crew.active_anomaly
                                .map(|a| a.label())
                                .unwrap_or("NOMINAL BASELINE"),
                        )
                        .size(15.0)
                        .strong()
                        .color(text_col),
                    );
                    ui.label(
                        RichText::new(format!("{:.0} ({:.0}–{:.0})", crew.hr_bpm, lo, hi))
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .color(text_col),
                    );
                    ui.label(
                        RichText::new(format!("{:.2}", crew.shock_index))
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .color(text_col),
                    );
                    ui.label(
                        RichText::new(&crew.orthostatic_risk)
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .color(text_col),
                    );
                    ui.label(
                        RichText::new(&crew.sans_risk)
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .color(INK_PRIMARY),
                    );
                    ui.label(
                        RichText::new(format!("{:.2}", crew.blood_n2_idx.unwrap_or(1.08)))
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .color(text_col),
                    );
                    ui.label(
                        RichText::new(if crew.alerts.health_danger {
                            "BREACH"
                        } else {
                            "PASS"
                        })
                        .size(15.0)
                        .family(FontFamily::Monospace)
                        .strong()
                        .color(if crew.alerts.health_danger {
                            OUTLINE_ALERT
                        } else {
                            OUTLINE_VIEW
                        }),
                    );
                    ui.label(
                        RichText::new(if crew.alerts.hazard_danger {
                            "BREACH"
                        } else {
                            "PASS"
                        })
                        .size(15.0)
                        .family(FontFamily::Monospace)
                        .strong()
                        .color(if crew.alerts.hazard_danger {
                            OUTLINE_ALERT
                        } else {
                            OUTLINE_VIEW
                        }),
                    );
                    ui.label(
                        RichText::new(if crew.alerts.hub_danger {
                            "EXCURSION"
                        } else {
                            "IN RANGE"
                        })
                        .size(15.0)
                        .family(FontFamily::Monospace)
                        .strong()
                        .color(if crew.alerts.hub_danger {
                            OUTLINE_ALERT
                        } else {
                            OUTLINE_VIEW
                        }),
                    );
                    ui.end_row();
                }
            });
    });
}

// ============================================================================
// MODULE 3: MEDICAL & INCIDENT LOG
// ============================================================================
fn render_medical_and_incident_module(ui: &mut egui::Ui, app: &mut HubApp) {
    let time_sec = ui.input(|i| i.time);
    render_module_header_bar(ui, app, HubModule::MedicalAndIncidentLog, time_sec);
    ui.add_space(12.0);

    // 1. Flight Surgeon Incidents Table
    let inc_border = if app.snapshot.unacked_incidents > 0 {
        OUTLINE_ALERT
    } else {
        OUTLINE_VIEW
    };
    let mut toggle_action: Option<(i64, bool)> = None;
    let mut log_new_incident = false;

    section_frame(inc_border).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "FLIGHT SURGEON INCIDENT LOG (UNACKNOWLEDGED: {})",
                    app.snapshot.unacked_incidents
                ))
                .size(22.0)
                .strong()
                .color(INK_PRIMARY),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let add_btn = egui::Button::new(
                    RichText::new("RECORD CURRENT TELEMETRY SNAPSHOT AS INCIDENT")
                        .size(15.0)
                        .strong()
                        .color(INK_PRIMARY),
                )
                .min_size(Vec2::new(340.0, 34.0))
                .fill(BG_PRIMARY)
                .stroke(Stroke::new(BORDER_DESKTOP, OUTLINE_INPUT));
                if ui.add(add_btn).clicked() {
                    log_new_incident = true;
                }
            });
        });
        ui.add_space(8.0);

        egui::Grid::new("surgeon_incidents_grid")
            .striped(true)
            .spacing(Vec2::new(14.0, 8.0))
            .show(ui, |ui| {
                for hdr in [
                    "ID",
                    "CREW",
                    "UTC TIMESTAMP",
                    "HAZARD TYPE",
                    "INJURY / CLINICAL DETAILS",
                    "MEDICATION",
                    "SURGEON NOTES",
                    "STATUS / ACTION",
                ] {
                    ui.label(RichText::new(hdr).size(15.0).strong().color(INK_PRIMARY));
                }
                ui.end_row();

                for inc in &app.snapshot.surgeon_incidents {
                    let acked = inc.surgeon_ack != 0;
                    let col = if acked { INK_PRIMARY } else { OUTLINE_ALERT };

                    ui.label(
                        RichText::new(format!("#{}", inc.incident_id))
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .strong()
                            .color(col),
                    );
                    ui.label(
                        RichText::new(&inc.astronaut_id)
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .strong()
                            .color(col),
                    );
                    ui.label(
                        RichText::new(&inc.created_at_utc)
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .color(INK_PRIMARY),
                    );
                    ui.label(
                        RichText::new(&inc.hazard_type)
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .strong()
                            .color(col),
                    );
                    ui.label(
                        RichText::new(inc.injury_details.as_deref().unwrap_or("—"))
                            .size(15.0)
                            .color(INK_PRIMARY),
                    );
                    ui.label(
                        RichText::new(inc.medication_administered.as_deref().unwrap_or("—"))
                            .size(15.0)
                            .color(INK_PRIMARY),
                    );
                    ui.label(
                        RichText::new(inc.medical_notes.as_deref().unwrap_or("—"))
                            .size(15.0)
                            .color(INK_PRIMARY),
                    );

                    let btn_label = if acked {
                        "RESOLVED (REOPEN)"
                    } else {
                        "ACKNOWLEDGE / RESOLVE"
                    };
                    let btn_border = if acked { OUTLINE_VIEW } else { OUTLINE_INPUT };
                    let act_btn = egui::Button::new(
                        RichText::new(btn_label)
                            .size(15.0)
                            .strong()
                            .color(INK_PRIMARY),
                    )
                    .min_size(Vec2::new(190.0, 30.0))
                    .fill(if acked { BG_PRIMARY } else { BG_ACTIVE })
                    .stroke(Stroke::new(2.5, btn_border));
                    if ui.add(act_btn).clicked() {
                        toggle_action = Some((inc.incident_id, !acked));
                    }
                    ui.end_row();
                }
            });
    });

    if log_new_incident {
        app.log_active_alert_incident();
    }
    if let Some((inc_id, next_resolved)) = toggle_action {
        app.toggle_incident_resolved(inc_id, next_resolved);
    }

    ui.add_space(12.0);

    // 2. Batch Lab & Microbiome Reports (hub_lab_reports)
    section_frame(OUTLINE_VIEW).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label(
            RichText::new("BATCH IMMUNE & MICROBIOME LAB REPORTS (NASA OSD DATASETS)")
                .size(22.0)
                .strong()
                .color(INK_PRIMARY),
        );
        ui.add_space(8.0);

        egui::Grid::new("hub_lab_reports_grid")
            .striped(true)
            .spacing(Vec2::new(16.0, 8.0))
            .show(ui, |ui| {
                for hdr in [
                    "REPORT ID",
                    "SUBJECT / SURFACE",
                    "DATASET",
                    "DOMAIN",
                    "SAMPLE DATE",
                    "BIOMARKER",
                    "MEASURED VALUE",
                    "STATUS",
                ] {
                    ui.label(RichText::new(hdr).size(15.0).strong().color(INK_PRIMARY));
                }
                ui.end_row();

                for rep in &app.snapshot.lab_reports {
                    let elevated = rep.status_flag != "OLAĞAN";
                    let col = if rep.status_flag == "CRITICAL" {
                        OUTLINE_ALERT
                    } else if elevated {
                        OUTLINE_INPUT
                    } else {
                        INK_PRIMARY
                    };

                    ui.label(
                        RichText::new(format!("LAB-{:02}", rep.report_id))
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .color(INK_PRIMARY),
                    );
                    ui.label(
                        RichText::new(rep.astronaut_id.as_deref().unwrap_or("CABIN SWAB"))
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .strong()
                            .color(INK_PRIMARY),
                    );
                    ui.label(
                        RichText::new(&rep.source_dataset)
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .color(INK_PRIMARY),
                    );
                    ui.label(
                        RichText::new(&rep.domain)
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .color(INK_PRIMARY),
                    );
                    ui.label(
                        RichText::new(&rep.sample_date)
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .color(INK_PRIMARY),
                    );
                    ui.label(
                        RichText::new(&rep.biomarker_name)
                            .size(15.0)
                            .strong()
                            .color(INK_PRIMARY),
                    );
                    ui.label(
                        RichText::new(format!("{:.2} {}", rep.measured_value, rep.unit))
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .strong()
                            .color(col),
                    );
                    ui_components::status_badge(
                        ui,
                        match rep.status_flag.as_str() {
                            "CRITICAL" => StatusLevel::Critical,
                            "ELEVATED" => StatusLevel::Caution,
                            _ => StatusLevel::Nominal,
                        },
                        &rep.status_flag,
                    );
                    ui.end_row();
                }
            });
    });

    ui.add_space(12.0);

    // 2B. NASA Historical Reference & Research Datasets (Non-Live)
    section_frame(OUTLINE_VIEW).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label(
            RichText::new("NASA HISTORICAL REFERENCE & RESEARCH DATA (NON-LIVE)")
                .size(22.0)
                .strong()
                .color(INK_PRIMARY),
        );
        ui.add_space(6.0);

        let rad_n = app.snapshot.radlab_series.len();
        let (rad_avg, rad_max) = if rad_n > 0 {
            let sum: f64 = app
                .snapshot
                .radlab_series
                .iter()
                .map(|r| r.absorbed_dose_rate_ugy_hr)
                .sum();
            let mx = app
                .snapshot
                .radlab_series
                .iter()
                .map(|r| r.absorbed_dose_rate_ugy_hr)
                .fold(0.0_f64, f64::max);
            (sum / rad_n as f64, mx)
        } else {
            (0.0, 0.0)
        };

        let eda_n = app.snapshot.eda_environment.len();
        let eda_co2_avg = if eda_n > 0 {
            let vals: Vec<f64> = app
                .snapshot
                .eda_environment
                .iter()
                .filter_map(|e| e.co2_iss_ppm)
                .collect();
            if vals.is_empty() {
                0.0
            } else {
                vals.iter().sum::<f64>() / vals.len() as f64
            }
        } else {
            0.0
        };

        ui.label(
            RichText::new(format!(
                "RADLAB 2013-01-01 (Liulin-5-D3): {} pts | AVG {:.2} uGy/hr | MAX {:.2} uGy/hr    ||    EDA RR-1 2014-09-25: {} pts | ISS AVG CO2 {:.0} ppm    ||    PROVENANCE: {} SHA256-VERIFIED FILES",
                rad_n,
                rad_avg,
                rad_max,
                eda_n,
                eda_co2_avg,
                app.snapshot.dataset_provenance.len()
            ))
            .size(14.0)
            .family(FontFamily::Monospace)
            .color(INK_PRIMARY),
        );
        ui.add_space(8.0);

        egui::Grid::new("nasa_research_obs_grid")
            .striped(true)
            .spacing(Vec2::new(14.0, 6.0))
            .show(ui, |ui| {
                for hdr in [
                    "STUDY",
                    "SUBJECT",
                    "TIMEPOINT",
                    "BIOMARKER",
                    "VALUE",
                    "REF RANGE",
                    "STATUS",
                ] {
                    ui.label(RichText::new(hdr).size(14.0).strong().color(INK_PRIMARY));
                }
                ui.end_row();

                for obs in app.snapshot.research_labs.iter().take(12) {
                    let col = match obs.status_flag.as_str() {
                        "ABOVE_REF" | "BELOW_REF" => OUTLINE_INPUT,
                        _ => INK_PRIMARY,
                    };
                    ui.label(
                        RichText::new(&obs.study_id)
                            .size(14.0)
                            .family(FontFamily::Monospace)
                            .color(INK_PRIMARY),
                    );
                    ui.label(
                        RichText::new(&obs.subject_code)
                            .size(14.0)
                            .family(FontFamily::Monospace)
                            .strong()
                            .color(INK_PRIMARY),
                    );
                    ui.label(
                        RichText::new(&obs.timepoint)
                            .size(14.0)
                            .family(FontFamily::Monospace)
                            .color(INK_PRIMARY),
                    );
                    ui.label(
                        RichText::new(&obs.biomarker_name)
                            .size(14.0)
                            .color(INK_PRIMARY),
                    );
                    ui.label(
                        RichText::new(format!("{:.2} {}", obs.measured_value, obs.unit))
                            .size(14.0)
                            .family(FontFamily::Monospace)
                            .strong()
                            .color(col),
                    );
                    let ref_txt = match (obs.ref_min, obs.ref_max) {
                        (Some(lo), Some(hi)) => format!("{lo:.1}..{hi:.1}"),
                        _ => "-".to_string(),
                    };
                    ui.label(
                        RichText::new(ref_txt)
                            .size(14.0)
                            .family(FontFamily::Monospace)
                            .color(INK_PRIMARY),
                    );
                    ui.label(
                        RichText::new(&obs.status_flag)
                            .size(14.0)
                            .family(FontFamily::Monospace)
                            .color(col),
                    );
                    ui.end_row();
                }
            });
    });

    ui.add_space(12.0);

    // 3. Daily Sleep Actigraphy Summary
    section_frame(OUTLINE_VIEW).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label(
            RichText::new("DAILY SLEEP ACTIGRAPHY SUMMARY")
                .size(22.0)
                .strong()
                .color(INK_PRIMARY),
        );
        ui.add_space(8.0);

        egui::Grid::new("sleep_summary_grid")
            .striped(true)
            .spacing(Vec2::new(18.0, 8.0))
            .show(ui, |ui| {
                for hdr in [
                    "CREW ID",
                    "MISSION DAY",
                    "TOTAL SLEEP (MIN)",
                    "TARGET (MIN)",
                    "SLEEP EFFICIENCY",
                    "AWAKENINGS",
                    "COMPUTED AT UTC",
                ] {
                    ui.label(RichText::new(hdr).size(15.0).strong().color(INK_PRIMARY));
                }
                ui.end_row();

                for sl in &app.snapshot.sleep_summaries {
                    ui.label(
                        RichText::new(&sl.astronaut_id)
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .strong()
                            .color(INK_PRIMARY),
                    );
                    ui.label(
                        RichText::new(format!("DAY {}", sl.mission_day))
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .color(INK_PRIMARY),
                    );
                    ui.label(
                        RichText::new(format!("{:.0} MIN", sl.total_sleep_min))
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .strong()
                            .color(INK_PRIMARY),
                    );
                    ui.label(
                        RichText::new(format!("{} MIN", sl.target_sleep_min))
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .color(INK_PRIMARY),
                    );
                    ui.label(
                        RichText::new(format!("{:.1}%", sl.sleep_efficiency_pct.unwrap_or(90.0)))
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .color(INK_PRIMARY),
                    );
                    ui.label(
                        RichText::new(format!("{}", sl.awakenings.unwrap_or(1)))
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .color(INK_PRIMARY),
                    );
                    ui.label(
                        RichText::new(&sl.computed_at_utc)
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .color(INK_PRIMARY),
                    );
                    ui.end_row();
                }
            });
    });

    ui.add_space(12.0);

    // 4. Daily SANS & Osteoporosis Histories Side-by-Side
    ui.columns(2, |cols| {
        section_frame(OUTLINE_VIEW).show(&mut cols[0], |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                RichText::new("DAILY SANS RISK HISTORY")
                    .size(22.0)
                    .strong()
                    .color(INK_PRIMARY),
            );
            ui.add_space(8.0);
            egui::Grid::new("sans_history_grid")
                .striped(true)
                .spacing(Vec2::new(12.0, 8.0))
                .show(ui, |ui| {
                    for hdr in [
                        "CREW", "DAY", "NA (MG)", "AVG CO2", "MAX CO2", "SCORE", "RISK",
                    ] {
                        ui.label(RichText::new(hdr).size(15.0).strong().color(INK_PRIMARY));
                    }
                    ui.end_row();
                    for row in &app.snapshot.sans_history {
                        ui.label(
                            RichText::new(&row.astronaut_id)
                                .size(15.0)
                                .family(FontFamily::Monospace)
                                .strong(),
                        );
                        ui.label(
                            RichText::new(format!("{}", row.mission_day))
                                .size(15.0)
                                .family(FontFamily::Monospace),
                        );
                        ui.label(
                            RichText::new(format!("{:.0}", row.sodium_mg_total))
                                .size(15.0)
                                .family(FontFamily::Monospace),
                        );
                        ui.label(
                            RichText::new(format!("{:.0}", row.avg_cabin_co2_ppm))
                                .size(15.0)
                                .family(FontFamily::Monospace),
                        );
                        ui.label(
                            RichText::new(format!("{:.0}", row.max_cabin_co2_ppm))
                                .size(15.0)
                                .family(FontFamily::Monospace),
                        );
                        ui.label(
                            RichText::new(format!("{:.1}", row.sans_score))
                                .size(15.0)
                                .family(FontFamily::Monospace),
                        );
                        ui.label(
                            RichText::new(&row.sans_risk)
                                .size(15.0)
                                .family(FontFamily::Monospace)
                                .strong(),
                        );
                        ui.end_row();
                    }
                });
        });

        section_frame(OUTLINE_VIEW).show(&mut cols[1], |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                RichText::new("DAILY OSTEOPOROSIS HISTORY")
                    .size(22.0)
                    .strong()
                    .color(INK_PRIMARY),
            );
            ui.add_space(8.0);
            egui::Grid::new("osteo_history_grid")
                .striped(true)
                .spacing(Vec2::new(12.0, 8.0))
                .show(ui, |ui| {
                    for hdr in [
                        "CREW", "DAY", "EX IDX", "CA (MG)", "D3 (IU)", "PROT", "RISK",
                    ] {
                        ui.label(RichText::new(hdr).size(15.0).strong().color(INK_PRIMARY));
                    }
                    ui.end_row();
                    for row in &app.snapshot.osteo_history {
                        ui.label(
                            RichText::new(&row.astronaut_id)
                                .size(15.0)
                                .family(FontFamily::Monospace)
                                .strong(),
                        );
                        ui.label(
                            RichText::new(format!("{}", row.mission_day))
                                .size(15.0)
                                .family(FontFamily::Monospace),
                        );
                        ui.label(
                            RichText::new(format!("{:.0}", row.exercise_load_idx))
                                .size(15.0)
                                .family(FontFamily::Monospace),
                        );
                        ui.label(
                            RichText::new(format!("{:.0}", row.calcium_mg_total))
                                .size(15.0)
                                .family(FontFamily::Monospace),
                        );
                        ui.label(
                            RichText::new(format!("{:.0}", row.vit_d3_iu_total))
                                .size(15.0)
                                .family(FontFamily::Monospace),
                        );
                        ui.label(
                            RichText::new(format!("{:.1}", row.protection_score))
                                .size(15.0)
                                .family(FontFamily::Monospace),
                        );
                        ui.label(
                            RichText::new(&row.osteo_risk)
                                .size(15.0)
                                .family(FontFamily::Monospace)
                                .strong(),
                        );
                        ui.end_row();
                    }
                });
        });
    });
}

// ============================================================================
// MODULE 4: FOOD CATALOG & SYNC
// ============================================================================
fn render_food_and_sync_module(ui: &mut egui::Ui, app: &mut HubApp, time_sec: f64) {
    render_module_header_bar(ui, app, HubModule::FoodCatalogAndSync, time_sec);
    ui.add_space(12.0);

    // 1. LAN Sync Telemetry & Packet Injector
    let mut sim_pkt_clicked = false;
    section_frame(if app.snapshot.lan_ready {
        OUTLINE_VIEW
    } else {
        OUTLINE_ALERT
    })
    .show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            let (comm_rect, _) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover());
            icons::draw_comms_icon(
                ui.painter(),
                comm_rect.center(),
                12.0,
                INK_PRIMARY,
                app.snapshot.lan_ready,
            );
            ui.label(
                RichText::new("HABITAT LAN TELEMETRY SERVER & BIOMON SYNC")
                    .size(22.0)
                    .strong()
                    .color(INK_PRIMARY),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let sim_btn = egui::Button::new(
                    RichText::new("SIMULATE INBOUND BIOMON PACKET")
                        .size(15.0)
                        .strong()
                        .color(INK_PRIMARY),
                )
                .min_size(Vec2::new(280.0, 36.0))
                .fill(BG_ACTIVE)
                .stroke(Stroke::new(BORDER_DESKTOP, OUTLINE_INPUT));
                if ui.add(sim_btn).clicked() {
                    sim_pkt_clicked = true;
                }
            });
        });
        ui.add_space(8.0);
        ui.columns(4, |c| {
            ui_components::stat_card(
                &mut c[0],
                84.0,
                "LISTENING ENDPOINT",
                app.lan_server.bind_addr(),
                INK_PRIMARY,
                if app.snapshot.lan_ready {
                    "BIND STATUS: READY"
                } else {
                    "BIND STATUS: OFFLINE"
                },
                CardTheme::COMMS,
            );
            ui_components::stat_card(
                &mut c[1],
                84.0,
                "ACTIVE SUIT CLIENTS",
                &format!("{}", app.snapshot.connected_clients),
                INK_PRIMARY,
                "TCP STREAM SESSIONS",
                CardTheme::COMMS,
            );
            ui_components::stat_card(
                &mut c[2],
                84.0,
                "PACKETS RECEIVED",
                &format!("{}", app.snapshot.packets_received),
                INK_PRIMARY,
                "JSONL FRAMES",
                CardTheme::COMMS,
            );
            ui_components::stat_card(
                &mut c[3],
                84.0,
                "LAST SYNC FRAME",
                &app.snapshot.last_packet_info,
                INK_PRIMARY,
                "REALTIME TELEMETRY QUEUE",
                CardTheme::COMMS,
            );
        });
    });

    if sim_pkt_clicked {
        app.simulate_inbound_biomon_packet();
    }

    ui.add_space(12.0);

    // 2. ISS Space Food Catalog Table
    section_frame(OUTLINE_VIEW).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            let (drop_rect, _) = ui.allocate_exact_size(Vec2::splat(26.0), Sense::hover());
            icons::draw_droplet_icon(ui.painter(), drop_rect.center(), 11.0, INK_PRIMARY);
            ui.label(
                RichText::new("ISS SPACE FOOD & HYDRATION CATALOG (ALL NUTRITIONAL COLUMNS)")
                    .size(22.0)
                    .strong()
                    .color(INK_PRIMARY),
            );
        });
        ui.add_space(8.0);

        egui::Grid::new("iss_food_catalog_grid")
            .striped(true)
            .spacing(Vec2::new(14.0, 8.0))
            .show(ui, |ui| {
                for hdr in [
                    "FOOD ID",
                    "ITEM NAME",
                    "CATEGORY",
                    "KCAL",
                    "PROTEIN (G)",
                    "SODIUM (MG)",
                    "POTASSIUM (MG)",
                    "CALCIUM (MG)",
                    "VIT D3 (IU)",
                    "FLUID (ML)",
                    "LOW NA",
                ] {
                    ui.label(RichText::new(hdr).size(15.0).strong().color(INK_PRIMARY));
                }
                ui.end_row();

                for item in &app.snapshot.food_catalog {
                    ui.label(
                        RichText::new(&item.food_id)
                            .size(15.0)
                            .family(FontFamily::Monospace)
                            .strong(),
                    );
                    ui.label(RichText::new(&item.item_name).size(15.0).strong());
                    ui.label(
                        RichText::new(&item.category)
                            .size(15.0)
                            .family(FontFamily::Monospace),
                    );
                    ui.label(
                        RichText::new(format!("{:.0}", item.kcal))
                            .size(15.0)
                            .family(FontFamily::Monospace),
                    );
                    ui.label(
                        RichText::new(format!("{:.1}", item.protein_g))
                            .size(15.0)
                            .family(FontFamily::Monospace),
                    );
                    ui.label(
                        RichText::new(format!("{:.0}", item.sodium_mg))
                            .size(15.0)
                            .family(FontFamily::Monospace),
                    );
                    ui.label(
                        RichText::new(format!("{:.0}", item.potassium_mg))
                            .size(15.0)
                            .family(FontFamily::Monospace),
                    );
                    ui.label(
                        RichText::new(format!("{:.0}", item.calcium_mg))
                            .size(15.0)
                            .family(FontFamily::Monospace),
                    );
                    ui.label(
                        RichText::new(format!("{:.0}", item.vit_d3_iu))
                            .size(15.0)
                            .family(FontFamily::Monospace),
                    );
                    ui.label(
                        RichText::new(format!("{:.0}", item.fluid_ml))
                            .size(15.0)
                            .family(FontFamily::Monospace),
                    );
                    ui.label(
                        RichText::new(if item.is_low_sodium != 0 {
                            "YES (SANS SAFE)"
                        } else {
                            "NO (HIGH NA)"
                        })
                        .size(15.0)
                        .family(FontFamily::Monospace)
                        .strong()
                        .color(if item.is_low_sodium != 0 {
                            OUTLINE_VIEW
                        } else {
                            OUTLINE_INPUT
                        }),
                    );
                    ui.end_row();
                }
            });
    });

    ui.add_space(12.0);

    // 3. Synced Crew Manual Input History & Mission Ephemeris Table
    ui.columns(2, |cols| {
        section_frame(OUTLINE_VIEW).show(&mut cols[0], |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                RichText::new("SYNCED CREW INPUT HISTORY")
                    .size(22.0)
                    .strong()
                    .color(INK_PRIMARY),
            );
            ui.add_space(8.0);
            egui::Grid::new("crew_input_history_grid")
                .striped(true)
                .spacing(Vec2::new(12.0, 8.0))
                .show(ui, |ui| {
                    for hdr in ["CREW", "DAY", "TYPE", "REF", "PCT", "NOTES"] {
                        ui.label(RichText::new(hdr).size(15.0).strong().color(INK_PRIMARY));
                    }
                    ui.end_row();
                    for rec in &app.snapshot.crew_inputs {
                        ui.label(
                            RichText::new(&rec.astronaut_id)
                                .size(15.0)
                                .family(FontFamily::Monospace)
                                .strong(),
                        );
                        ui.label(
                            RichText::new(format!("D{}", rec.mission_day))
                                .size(15.0)
                                .family(FontFamily::Monospace),
                        );
                        ui.label(
                            RichText::new(&rec.input_type)
                                .size(15.0)
                                .family(FontFamily::Monospace),
                        );
                        ui.label(
                            RichText::new(rec.reference_code.as_deref().unwrap_or("—"))
                                .size(15.0)
                                .family(FontFamily::Monospace),
                        );
                        ui.label(
                            RichText::new(
                                rec.consumption_pct
                                    .map(|p| format!("{}%", p))
                                    .unwrap_or_else(|| "—".to_string()),
                            )
                            .size(15.0)
                            .family(FontFamily::Monospace),
                        );
                        ui.label(RichText::new(rec.notes.as_deref().unwrap_or("—")).size(15.0));
                        ui.end_row();
                    }
                });
        });

        section_frame(OUTLINE_VIEW).show(&mut cols[1], |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                RichText::new("EARTH-MARS EPHEMERIS & LIVE OWLT")
                    .size(22.0)
                    .strong()
                    .color(INK_PRIMARY),
            );
            ui.add_space(6.0);

            let t_frac = ((time_sec * 0.05).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
            let start_au = app
                .snapshot
                .ephemeris_all
                .first()
                .map(|e| e.distance_au)
                .unwrap_or(1.42);
            let end_au = app
                .snapshot
                .ephemeris_all
                .get(1)
                .map(|e| e.distance_au)
                .unwrap_or(1.43);
            let (live_au, live_owlt) = calc::interpolate_ephemeris(start_au, end_au, t_frac);

            ui.label(
                RichText::new(format!(
                    "INTERPOLATED LIVE: {:.4} AU  |  ONE-WAY DELAY: {:.3} MIN",
                    live_au, live_owlt
                ))
                .size(16.0)
                .family(FontFamily::Monospace)
                .strong()
                .color(INK_PRIMARY),
            );
            ui.add_space(6.0);

            egui::Grid::new("ephemeris_all_grid")
                .striped(true)
                .spacing(Vec2::new(14.0, 8.0))
                .show(ui, |ui| {
                    for hdr in ["DATE", "DAY", "DIST (AU)", "OWLT (MIN)", "SOLAR CONJ"] {
                        ui.label(RichText::new(hdr).size(15.0).strong().color(INK_PRIMARY));
                    }
                    ui.end_row();

                    for (idx, ep) in app.snapshot.ephemeris_all.iter().enumerate() {
                        let conj = ep.distance_au > 2.35;
                        ui.label(
                            RichText::new(&ep.mission_date)
                                .size(15.0)
                                .family(FontFamily::Monospace)
                                .strong(),
                        );
                        ui.label(
                            RichText::new(format!("DAY {}", idx + 1))
                                .size(15.0)
                                .family(FontFamily::Monospace),
                        );
                        ui.label(
                            RichText::new(format!("{:.3} AU", ep.distance_au))
                                .size(15.0)
                                .family(FontFamily::Monospace),
                        );
                        ui.label(
                            RichText::new(format!("{:.2} MIN", ep.one_way_light_min))
                                .size(15.0)
                                .family(FontFamily::Monospace),
                        );
                        ui.label(
                            RichText::new(if conj { "CONJUNCTION" } else { "NOMINAL LINK" })
                                .size(15.0)
                                .family(FontFamily::Monospace)
                                .color(if conj { OUTLINE_ALERT } else { OUTLINE_VIEW }),
                        );
                        ui.end_row();
                    }
                });
        });
    });
}
