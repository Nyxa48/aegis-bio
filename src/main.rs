mod biomon;

use aegis_bio_demo::{heading_font, install_aerospace_fonts};
use biomon::state::{BiomonState, Hazard, Mode, NavWaypoint, Phase, Repairability, SuitFault};
use biomon::theme::{
    lerp_color, palette, CardColors, ColorTheme, ColorVision, PanelId, StatusLevel,
    TextScalePreset, Tokens,
};
use biomon::widgets::{
    brightness_slider, color_button_h, draw_corridor_bar, draw_heart_icon, draw_refresh_icon,
    draw_signal_bars, draw_status_shape, draw_suit_schematic, draw_working_compass, fit_16_by_9,
    gauge_pill, stat_card_h, status_chip,
};
use eframe::egui::{
    self, Align, Align2, Color32, CursorIcon, FontFamily, FontId, Frame, Id, Layout, Margin,
    Order, Pos2, Rect, RichText, Rounding, Sense, Stroke, Ui, Vec2,
};
use std::time::{Duration, Instant};

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 540.0])
            .with_min_inner_size([640.0, 360.0])
            .with_title("AEGIS-BIO // Wrist Biomon"),
        ..Default::default()
    };

    eframe::run_native(
        "AEGIS-BIO Wrist Biomon",
        options,
        Box::new(|cc| Box::new(BiomonApp::new(cc))),
    )
}

// ───────────────────────── Application Shell ─────────────────────────
struct BiomonApp {
    state: BiomonState,
    last_tick: Instant,
    prng: u64,
}

impl BiomonApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_aerospace_fonts(&cc.egui_ctx);
        let app = Self {
            state: BiomonState::default(),
            last_tick: Instant::now(),
            prng: 0xAE61_5B10_2026_1114,
        };
        let init_t = Tokens::resolve(
            app.state.a11y,
            Rect::from_min_size(Pos2::ZERO, Vec2::new(960.0, 540.0)),
        );
        app.apply_style(&cc.egui_ctx, init_t);
        app
    }

    fn apply_style(&self, ctx: &egui::Context, t: Tokens) {
        ctx.style_mut(|s| {
            s.spacing.item_spacing = Vec2::splat(t.sp(6.0).max(4.0));
            s.spacing.button_padding = Vec2::new(t.sp(10.0), t.sp(6.0));
            s.interaction.selectable_labels = false;
        });
        let mut v = egui::Visuals::dark();
        v.panel_fill = t.outer_bg;
        v.window_fill = t.screen_bg;
        v.window_shadow = egui::epaint::Shadow::NONE;
        v.popup_shadow = egui::epaint::Shadow::NONE;
        v.override_text_color = Some(t.text_main);
        let r = Rounding::same(t.sp(8.0).max(5.0));
        v.widgets.noninteractive.rounding = r;
        v.widgets.inactive.rounding = r;
        v.widgets.hovered.rounding = r;
        v.widgets.active.rounding = r;
        ctx.set_visuals(v);
    }

    fn next_rand(&mut self) -> u64 {
        self.prng = self
            .prng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.prng >> 32
    }

    fn tick_1hz(&mut self) {
        if self.last_tick.elapsed() < Duration::from_secs(1) {
            return;
        }
        self.last_tick = Instant::now();

        let base = self.state.phase.base_hr()
            + if self.state.hazard == Hazard::Hypoxia { 14 } else { 0 };
        let jitter = (self.next_rand() % 3) as i32 - 1;
        self.state.live_hr = base + jitter;

        // Realistic gentle wrist heading drift (live working compass)
        let heading_step = if self.state.phase == Phase::Eva {
            ((self.next_rand() % 7) as f32) - 3.0
        } else {
            ((self.next_rand() % 3) as f32) - 1.0
        };
        self.state.compass_heading_deg = (self.state.compass_heading_deg + heading_step + 360.0) % 360.0;

        if self.state.phase == Phase::Eva && (self.next_rand() % 2 == 1) {
            self.state.distance_m += 1;
        }

        if !self.state.online {
            self.state.buffered_packets += 1;
        }
    }
}


impl eframe::App for BiomonApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.tick_1hz();
        // Clean 250ms cadence without stifling continuous 30fps loop; pointer clicks repaint immediately
        ctx.request_repaint_after(Duration::from_millis(250));

        egui::CentralPanel::default()
            .frame(Frame::none().fill(palette::VOID_950).inner_margin(Margin::same(0.0)))
            .show(ctx, |ui| {
                let avail = ui.available_rect_before_wrap();
                let margined = avail.shrink2(Vec2::new(avail.width() * 0.015, avail.height() * 0.015));
                let screen_rect = fit_16_by_9(margined);

                let t = Tokens::resolve(self.state.a11y, screen_rect);
                self.apply_style(ctx, t);

                let painter = ui.painter();
                painter.rect_filled(avail, 0.0, t.outer_bg);
                painter.rect_filled(screen_rect, t.sp(14.0), t.screen_bg);
                painter.rect_stroke(screen_rect, t.sp(14.0), Stroke::new(t.stroke_w, t.hairline_strong));

                if let Some(panel) = self.state.active_panel {
                    self.render_detail_overlay(ui, ctx, screen_rect, panel, t);
                } else {
                    self.render_quadrant_grid(ui, ctx, screen_rect, t);
                }
            });
    }
}

impl BiomonApp {
    fn render_quadrant_grid(&mut self, ui: &mut Ui, ctx: &egui::Context, screen_rect: Rect, t: Tokens) {
        let center = screen_rect.center();
        let inner_r = (screen_rect.height() * 0.17).max(48.0);
        let ring_r = inner_r + t.sp(7.0);

        let pointer_in_cycle = ctx
            .input(|i| i.pointer.hover_pos())
            .is_some_and(|pos| pos.distance(center) <= ring_r);

        let gap = t.gap;
        let grid_rect = screen_rect.shrink(gap);
        let qw = (grid_rect.width() - gap) / 2.0;
        let qh = (grid_rect.height() - gap) / 2.0;

        let outer_pad = t.sp(14.0).max(10.0);
        let v_pad = t.sp(10.0).max(7.0);
        let clear_x = (ring_r * 0.85).max(outer_pad);
        let clear_y = (ring_r * 0.30).max(v_pad);

        let rects = [
            (
                PanelId::Health,
                Rect::from_min_size(grid_rect.min, Vec2::new(qw, qh)),
                Margin {
                    left: outer_pad,
                    right: clear_x,
                    top: v_pad,
                    bottom: clear_y,
                },
            ),
            (
                PanelId::Hub,
                Rect::from_min_size(
                    Pos2::new(grid_rect.min.x + qw + gap, grid_rect.min.y),
                    Vec2::new(qw, qh),
                ),
                Margin {
                    left: clear_x,
                    right: outer_pad,
                    top: v_pad,
                    bottom: clear_y,
                },
            ),
            (
                PanelId::Tech,
                Rect::from_min_size(
                    Pos2::new(grid_rect.min.x, grid_rect.min.y + qh + gap),
                    Vec2::new(qw, qh),
                ),
                Margin {
                    left: outer_pad,
                    right: clear_x,
                    top: clear_y,
                    bottom: v_pad,
                },
            ),
            (
                PanelId::Hazard,
                Rect::from_min_size(
                    Pos2::new(grid_rect.min.x + qw + gap, grid_rect.min.y + qh + gap),
                    Vec2::new(qw, qh),
                ),
                Margin {
                    left: clear_x,
                    right: outer_pad,
                    top: clear_y,
                    bottom: v_pad,
                },
            ),
        ];

        let card_r = t.sp(12.0).max(8.0);
        let suit_rep = self.state.overall_suit_repairability();
        let is_edit = self.state.mode == Mode::Input;

        for (panel_id, q_rect, pad) in rects {
            let is_alert = match panel_id {
                PanelId::Health => {
                    self.state.hazard == Hazard::Hypoxia || suit_rep == Repairability::AirlockRequired
                }
                PanelId::Hazard => self.state.hazard != Hazard::Nominal,
                _ => false,
            };

            let resp = ui.interact(q_rect, ui.id().with(("quad_btn", panel_id as u8)), Sense::click());
            let hovered = resp.hovered() && !pointer_in_cycle;
            if hovered {
                ctx.set_cursor_icon(CursorIcon::PointingHand);
            }

            let hover_t = ctx.animate_bool_with_time(Id::new(("quad_hover", panel_id as u8)), hovered, t.anim_secs);
            let base_c = t.card_colors(panel_id);
            let target_c = if is_alert {
                t.alert_card
            } else if is_edit {
                CardColors {
                    bg: lerp_color(base_c.bg, t.edit_card.bg, 0.48),
                    hover: lerp_color(base_c.hover, t.edit_card.hover, 0.48),
                    border: t.edit_card.border,
                    accent: lerp_color(base_c.accent, t.edit_card.accent, 0.45),
                }
            } else {
                base_c
            };

            let fill = lerp_color(target_c.bg, target_c.hover, hover_t);
            let border_col = lerp_color(target_c.border, target_c.accent, hover_t * 0.55);

            let painter = ui.painter();
            painter.rect_filled(q_rect, card_r, fill);
            painter.rect_stroke(q_rect, card_r, Stroke::new(t.stroke_w, border_col));

            let content_rect = Rect::from_min_max(
                Pos2::new(q_rect.min.x + pad.left, q_rect.min.y + pad.top),
                Pos2::new(q_rect.max.x - pad.right, q_rect.max.y - pad.bottom),
            );
            ui.allocate_ui_at_rect(content_rect, |ui| {
                self.render_quadrant_content(ui, panel_id, target_c.accent, t);
            });

            if resp.clicked() && !pointer_in_cycle {
                self.state.active_panel = Some(panel_id);
            }
        }

        // Dead-Center Circular Cycle Dial
        let alloc_pad = t.sp(4.0);
        egui::Area::new(Id::new("cycle_button"))
            .order(Order::Foreground)
            .fixed_pos(center - Vec2::splat(ring_r + alloc_pad))
            .show(ctx, |ui| {
                let (ring_rect, _) =
                    ui.allocate_exact_size(Vec2::splat((ring_r + alloc_pad) * 2.0), Sense::hover());
                let inner_rect =
                    Rect::from_center_size(ring_rect.center(), Vec2::splat(inner_r * 2.0));
                let resp = ui.interact(inner_rect, ui.id().with("cycle_inner"), Sense::click());
                if resp.hovered() {
                    ctx.set_cursor_icon(CursorIcon::PointingHand);
                }

                let c_hover = ctx.animate_bool_with_time(Id::new("cycle_hover"), resp.hovered(), t.anim_secs);
                let painter = ui.painter();
                let c = ring_rect.center();

                painter.circle(c, ring_r, t.screen_bg, Stroke::new(t.stroke_w, t.hairline));

                let (base_fill, hover_fill, ring_col, mode_lbl) = if is_edit {
                    (t.edit_card.bg, t.edit_card.hover, t.accent_input, "Edit mode")
                } else {
                    (t.raised_bg, t.raised_hover, t.accent_view, "View mode")
                };
                let fill_col = lerp_color(base_fill, hover_fill, c_hover);
                let icon_col = if is_edit { t.accent_input } else { t.text_main };

                painter.circle(c, inner_r, fill_col, Stroke::new(t.stroke_w + t.sp(1.0), ring_col));
                draw_refresh_icon(
                    painter,
                    Pos2::new(c.x, c.y - inner_r * 0.44),
                    inner_r * 0.14,
                    icon_col,
                    t,
                );

                let dial_scale = self.state.a11y.text_scale.factor().min(1.06);
                painter.text(
                    Pos2::new(c.x, c.y - inner_r * 0.08),
                    Align2::CENTER_CENTER,
                    "Cycle",
                    heading_font((inner_r * 0.22).clamp(11.0, 34.0) * dial_scale),
                    t.text_main,
                );
                painter.text(
                    Pos2::new(c.x, c.y + inner_r * 0.17),
                    Align2::CENTER_CENTER,
                    mode_lbl,
                    FontId::proportional((inner_r * 0.14).clamp(9.0, 22.0) * dial_scale),
                    ring_col,
                );
                painter.text(
                    Pos2::new(c.x, c.y + inner_r * 0.48),
                    Align2::CENTER_CENTER,
                    self.state.phase.name(),
                    FontId::monospace((inner_r * 0.14).clamp(8.5, 22.0) * dial_scale),
                    t.text_muted,
                );

                if resp.clicked() {
                    self.state.mode = match self.state.mode {
                        Mode::Main => Mode::Input,
                        Mode::Input => Mode::Main,
                    };
                }
            });
    }

    fn render_quadrant_content(&self, ui: &mut Ui, panel: PanelId, accent: Color32, t: Tokens) {
        let is_input = self.state.mode == Mode::Input;
        let ration_short = self.state.ration_short();
        let rep = self.state.overall_suit_repairability();

        let title = match panel {
            PanelId::Health => "Health & suit",
            PanelId::Hub => "Hub & nav",
            PanelId::Tech => "Comms & display",
            PanelId::Hazard => "Hazard & env",
        };

        // Clean compact header without 01/02/03/04 badges
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(title)
                    .font(heading_font(t.sz(14.0)))
                    .color(t.text_main),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let (tag_txt, tag_col) = if is_input {
                    ("Edit", t.accent_input)
                } else {
                    ("Live", accent)
                };
                ui.label(RichText::new(tag_txt).size(t.sz(10.5)).color(tag_col));
            });
        });

        ui.add_space(t.sp(3.0));
        ui.painter().line_segment(
            [
                Pos2::new(ui.min_rect().left(), ui.cursor().top()),
                Pos2::new(ui.max_rect().right(), ui.cursor().top()),
            ],
            Stroke::new(1.0_f32, accent.gamma_multiply(0.30)),
        );

        // Full remaining space used directly (no bottom footer strip!)
        ui.allocate_ui_with_layout(
            ui.available_size(),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.vertical(|ui| match panel {
                    PanelId::Health => {
                        if !is_input {
                            let (lvl, suit_col, suit_txt) = if rep == Repairability::AirlockRequired
                                || self.state.hazard == Hazard::Hypoxia
                            {
                                (StatusLevel::Critical, t.critical, "Airlock req.")
                            } else if rep == Repairability::FieldRepairable {
                                (StatusLevel::Caution, t.caution, "Field repair")
                            } else if self.state.phase == Phase::ReEntry {
                                (StatusLevel::Caution, t.caution, "Fluid load")
                            } else {
                                (StatusLevel::Nominal, accent, "Suit intact")
                            };

                            ui.horizontal(|ui| {
                                draw_heart_icon(ui, t.sp(18.0), suit_col, t);
                                ui.label(
                                    RichText::new(format!("{}", self.state.live_hr))
                                        .family(FontFamily::Monospace)
                                        .size(t.sz(28.0))
                                        .color(t.text_main),
                                );
                                ui.label(
                                    RichText::new("bpm")
                                        .family(FontFamily::Monospace)
                                        .size(t.sz(11.0))
                                        .color(t.text_muted),
                                );
                            });
                            let (lo, hi) = self.state.phase.hr_bounds();
                            draw_corridor_bar(
                                ui,
                                (ui.available_width() * 0.84).max(t.sp(90.0)),
                                self.state.live_hr as f32,
                                lo as f32,
                                hi as f32,
                                suit_col,
                                t,
                            );
                            ui.add_space(t.sp(4.0));
                            ui.horizontal(|ui| {
                                status_chip(ui, lvl, suit_txt, suit_col, t);
                                ui.label(
                                    RichText::new(format!("· BP {}", self.state.phase.bp()))
                                        .family(FontFamily::Monospace)
                                        .size(t.sz(11.0))
                                        .color(t.text_muted),
                                );
                            });
                        } else {
                            status_chip(ui, StatusLevel::Caution, "Meal, fluids & suit", t.accent_input, t);
                            ui.add_space(t.sp(5.0));
                            ui.label(
                                RichText::new(format!(
                                    "{}% meal · {} mL · {}",
                                    self.state.meal_eaten_pct,
                                    self.state.water_ml,
                                    self.state.suit_fault.sub()
                                ))
                                .family(FontFamily::Monospace)
                                .size(t.sz(13.0))
                                .color(t.text_main),
                            );
                        }
                    }
                    PanelId::Hub => {
                        if !is_input {
                            // Full space split cleanly between Distance/Waypoint info and large working Compass!
                            let dial_sz = (ui.available_height() * 0.86).clamp(t.sp(52.0), t.sp(110.0));
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!("{}", self.state.distance_m))
                                                .family(FontFamily::Monospace)
                                                .size(t.sz(28.0))
                                                .color(t.text_main),
                                        );
                                        ui.label(
                                            RichText::new("m")
                                                .family(FontFamily::Monospace)
                                                .size(t.sz(12.0))
                                                .color(t.text_muted),
                                        );
                                    });
                                    ui.label(
                                        RichText::new(format!(
                                            "{} · {:03.0}° {}",
                                            self.state.waypoint.label(),
                                            self.state.waypoint.bearing_deg(),
                                            self.state.waypoint.cardinal()
                                        ))
                                        .family(FontFamily::Monospace)
                                        .size(t.sz(11.5))
                                        .color(t.text_body),
                                    );
                                    ui.label(
                                        RichText::new(format!("{} · {}", self.state.phase.name(), ration_short))
                                            .size(t.sz(11.5))
                                            .color(accent),
                                    );
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    draw_working_compass(
                                        ui,
                                        dial_sz,
                                        self.state.compass_heading_deg,
                                        self.state.waypoint.bearing_deg(),
                                        accent,
                                        false,
                                        t,
                                    );
                                });
                            });
                        } else {
                            status_chip(ui, StatusLevel::Caution, "Phase & waypoint", t.accent_input, t);
                            ui.add_space(t.sp(5.0));
                            ui.label(
                                RichText::new(format!("{} · {} {}", self.state.phase.name(), self.state.waypoint.label(), self.state.distance_m))
                                    .family(FontFamily::Monospace)
                                    .size(t.sz(13.5))
                                    .color(t.text_main),
                            );
                        }
                    }
                    PanelId::Tech => {
                        if !is_input {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    if self.state.online {
                                        status_chip(ui, StatusLevel::Nominal, "Online", t.nominal, t);
                                    } else {
                                        status_chip(
                                            ui,
                                            StatusLevel::Caution,
                                            &format!("Offline · {} pkts", self.state.buffered_packets),
                                            t.caution,
                                            t,
                                        );
                                    }
                                    ui.add_space(t.sp(3.0));
                                    if self.state.hazard != Hazard::Nominal {
                                        status_chip(ui, StatusLevel::Critical, "Protocol alert", t.critical, t);
                                    } else {
                                        ui.label(
                                            RichText::new("NASA-STD-3001 OK")
                                                .family(FontFamily::Monospace)
                                                .size(t.sz(11.5))
                                                .color(accent),
                                        );
                                    }
                                    ui.add_space(t.sp(2.0));
                                    ui.label(
                                        RichText::new(self.state.a11y.summary())
                                            .family(FontFamily::Monospace)
                                            .size(t.sz(10.5))
                                            .color(t.text_muted),
                                    );
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    draw_signal_bars(ui, t.sp(36.0), self.state.online, accent, t.caution, t);
                                });
                            });
                        } else {
                            status_chip(ui, StatusLevel::Caution, "Display & radio", t.accent_input, t);
                            ui.add_space(t.sp(5.0));
                            ui.label(
                                RichText::new(self.state.a11y.summary())
                                    .family(FontFamily::Monospace)
                                    .size(t.sz(13.0))
                                    .color(t.text_main),
                            );
                        }
                    }
                    PanelId::Hazard => {
                        if !is_input {
                            let (lvl, title_col) = match self.state.hazard {
                                Hazard::Nominal => (StatusLevel::Nominal, t.nominal),
                                Hazard::HighCo2 => (StatusLevel::Caution, t.caution),
                                Hazard::Radiation | Hazard::Hypoxia => (StatusLevel::Critical, t.critical),
                            };
                            status_chip(ui, lvl, self.state.hazard.title(), title_col, t);
                            ui.add_space(t.sp(7.0));
                            ui.horizontal(|ui| {
                                gauge_pill(
                                    ui,
                                    "Rad",
                                    self.state.hazard.rad_val(),
                                    self.state.hazard.rad_ratio(),
                                    self.state.hazard == Hazard::Radiation,
                                    t,
                                );
                                ui.add_space(t.sp(10.0));
                                gauge_pill(
                                    ui,
                                    "CO2",
                                    self.state.hazard.co2_val(),
                                    self.state.hazard.co2_ratio(),
                                    self.state.hazard == Hazard::HighCo2,
                                    t,
                                );
                                ui.add_space(t.sp(10.0));
                                gauge_pill(
                                    ui,
                                    "O2",
                                    self.state.hazard.o2(),
                                    self.state.hazard.o2_ratio(),
                                    self.state.hazard == Hazard::Hypoxia,
                                    t,
                                );
                            });
                        } else {
                            status_chip(ui, StatusLevel::Caution, "Hazard tests", t.accent_input, t);
                            ui.add_space(t.sp(5.0));
                            ui.label(
                                RichText::new(self.state.hazard.label())
                                    .size(t.sz(14.0))
                                    .color(t.text_main),
                            );
                        }
                    }
                });
            },
        );
    }

    fn render_detail_overlay(
        &mut self,
        ui: &mut Ui,
        ctx: &egui::Context,
        screen_rect: Rect,
        panel: PanelId,
        t: Tokens,
    ) {
        let is_input = self.state.mode == Mode::Input;
        let inner = screen_rect.shrink(t.sp(12.0));
        let card_c = t.card_colors(panel);

        if !is_input {
            // View screens are read-only: whole body is content, a tap anywhere returns to the grid.
            ui.allocate_ui_at_rect(inner, |ui| match panel {
                PanelId::Health => self.overlay_health(ui, ctx, false, card_c, t),
                PanelId::Hub => self.overlay_hub(ui, ctx, false, card_c, t),
                PanelId::Tech => self.overlay_tech(ui, ctx, false, card_c, t),
                PanelId::Hazard => self.overlay_hazard(ui, ctx, false, card_c, t),
            });
            let resp = ui.interact(screen_rect, Id::new("overlay_tap_return"), Sense::click());
            if resp.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if resp.clicked() {
                self.state.active_panel = None;
            }
            return;
        }

        // Edit screens: no title, controls fill the body, one extra-large full-width Return bar at the bottom.
        let gap = t.sp(8.0);
        let ret_h = (inner.height() * 0.19).clamp(t.button_h * 1.45, t.sp(120.0));
        let ret_rect = Rect::from_min_max(Pos2::new(inner.left(), inner.bottom() - ret_h), inner.max);
        let body_rect = Rect::from_min_max(inner.min, Pos2::new(inner.right(), ret_rect.top() - gap));

        ui.allocate_ui_at_rect(body_rect, |ui| match panel {
            PanelId::Health => self.overlay_health(ui, ctx, true, card_c, t),
            PanelId::Hub => self.overlay_hub(ui, ctx, true, card_c, t),
            PanelId::Tech => self.overlay_tech(ui, ctx, true, card_c, t),
            PanelId::Hazard => self.overlay_hazard(ui, ctx, true, card_c, t),
        });

        let resp = ui.interact(ret_rect, Id::new("overlay_return_btn"), Sense::click());
        if resp.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        let h_t = ctx.animate_bool_with_time(Id::new("ret_hover"), resp.hovered(), t.anim_secs);
        let fill = lerp_color(card_c.bg, card_c.hover, h_t);
        let p = ui.painter();
        p.rect(ret_rect, t.sp(9.0), fill, Stroke::new(t.stroke_w * 1.6, card_c.accent));
        let pref = (t.text_mult / t.ui_scale.max(0.01)).clamp(0.8, 1.5);
        p.text(
            ret_rect.center(),
            Align2::CENTER_CENTER,
            "←  Return",
            FontId::proportional((ret_h * 0.42 * pref).clamp(16.0, 48.0)),
            t.text_main,
        );
        if resp.clicked() {
            self.state.active_panel = None;
        }
    }

    fn overlay_health(
        &mut self,
        ui: &mut Ui,
        ctx: &egui::Context,
        is_input: bool,
        card_c: CardColors,
        t: Tokens,
    ) {
        let p = self.state.phase;
        let zones = self.state.suit_zones();
        let overall_rep = self.state.overall_suit_repairability();
        let body_h = ui.available_height();
        let gap = t.sp(8.0);

        if !is_input {
            let top_h = (body_h * 0.27).clamp(t.sp(58.0), t.sp(118.0));
            ui.columns(4, |c| {
                stat_card_h(&mut c[0], top_h, "Heart rate", &format!("{} bpm", self.state.live_hr), card_c.accent, p.hr_range(), card_c, t);
                stat_card_h(&mut c[1], top_h, "Blood pressure", p.bp(), t.text_main, "Optical", card_c, t);
                stat_card_h(&mut c[2], top_h, "Calorie deficit", &format!("{} kcal", self.state.calorie_deficit), t.caution, &format!("{:.1}/min", p.kcal_rate()), card_c, t);
                stat_card_h(&mut c[3], top_h, "Hydration", &format!("-{} mL", self.state.sweat_deficit_ml), t.accent_view, &format!("+{} mL in", self.state.water_ml), card_c, t);
            });

            ui.add_space(gap);

            // Remaining 100% height for Spacesuit Schematic + Suit Damage & Repair (zero rag_box filler!)
            let mid_h = (body_h - top_h - gap - t.sp(2.0)).max(t.sp(120.0));
            ui.columns(2, |cols| {
                let left_rect = cols[0].allocate_exact_size(Vec2::new(cols[0].available_width(), mid_h), Sense::hover()).0;
                cols[0].painter().rect(left_rect, t.sp(8.0), card_c.bg, Stroke::new(t.stroke_w, card_c.border));
                let left_in = left_rect.shrink(t.sp(10.0));
                cols[0].allocate_ui_at_rect(left_in, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Spacesuit schematic").font(heading_font(t.sz(13.0))).color(t.text_main));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let (lvl, col, msg) = match overall_rep {
                                Repairability::Intact => (StatusLevel::Nominal, t.nominal, "100% Intact"),
                                Repairability::FieldRepairable => (StatusLevel::Caution, t.caution, "Field repairable"),
                                Repairability::AirlockRequired => (StatusLevel::Critical, t.critical, "Airlock req."),
                            };
                            status_chip(ui, lvl, msg, col, t);
                        });
                    });
                    let schem_h = (ui.available_height() - t.sp(4.0)).max(t.sp(70.0));
                    ui.centered_and_justified(|ui| {
                        draw_suit_schematic(ui, schem_h, &zones, t);
                    });
                });

                let right_rect = cols[1].allocate_exact_size(Vec2::new(cols[1].available_width(), mid_h), Sense::hover()).0;
                cols[1].painter().rect(right_rect, t.sp(8.0), card_c.bg, Stroke::new(t.stroke_w, card_c.border));
                let right_in = right_rect.shrink(t.sp(10.0));
                cols[1].allocate_ui_at_rect(right_in, |ui| {
                    ui.label(RichText::new("Suit damage & repair").font(heading_font(t.sz(13.0))).color(t.text_main));
                    ui.add_space(t.sp(4.0));
                    let row_avail = (ui.available_height() - t.sp(6.0)).max(t.sp(75.0));
                    let pref = (t.text_mult / t.ui_scale.max(0.01)).clamp(0.8, 1.5);
                    let zone_sz = (row_avail / 5.0 * 0.30 * pref).clamp(11.0, 26.0);
                    let act_sz = (zone_sz * 0.82).max(10.0);
                    let row_step = (row_avail / 5.0 - zone_sz * 2.2).clamp(t.sp(2.0), t.sp(18.0));
                    for z in &zones {
                        let (lvl, col, badge) = match z.repairability {
                            Repairability::Intact => (StatusLevel::Nominal, t.nominal, "Intact"),
                            Repairability::FieldRepairable => (StatusLevel::Caution, t.caution, "Field repair"),
                            Repairability::AirlockRequired => (StatusLevel::Critical, t.critical, "Unrepairable"),
                        };
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(Vec2::splat(t.sp(8.0).max(6.0)), Sense::hover());
                            draw_status_shape(ui.painter(), r.center(), lvl, col, t);
                            ui.label(RichText::new(z.zone).size(zone_sz).color(t.text_main));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(
                                    RichText::new(format!("{}% · {}", z.integrity_pct, badge))
                                        .family(FontFamily::Monospace)
                                        .size(act_sz)
                                        .color(col),
                                );
                            });
                        });
                        if z.repairability != Repairability::Intact {
                            ui.label(RichText::new(z.action).size(act_sz).color(t.text_muted));
                        }
                        ui.add_space(row_step * 0.45);
                    }
                });
            });
        } else {
            // Non-scrolling heading-free 2-column Edit layout where buttons fill 100% height cleanly
            let left_rows = if overall_rep == Repairability::FieldRepairable { 5.0 } else { 4.0 };
            let left_btn_h = ((body_h - gap * (left_rows - 1.0)) / left_rows).max(t.sp(30.0));
            let right_btn_h = ((body_h - gap * 3.0) / 4.0).max(t.sp(30.0));

            ui.columns(2, |cols| {
                cols[0].vertical(|ui| {
                    for (i, &fault) in SuitFault::ALL.iter().enumerate() {
                        if color_button_h(ui, ctx, ("fault_btn", i), left_btn_h, fault.label(), fault.sub(), self.state.suit_fault == fault, card_c, t).clicked() {
                            self.state.suit_fault = fault;
                        }
                        if i < 3 || overall_rep == Repairability::FieldRepairable {
                            ui.add_space(gap);
                        }
                    }
                    if overall_rep == Repairability::FieldRepairable {
                        if color_button_h(ui, ctx, "edit_repair_btn", left_btn_h, "Apply EVA field repair", "Restore 100%", true, card_c, t).clicked() {
                            self.state.apply_field_repair();
                        }
                    }
                });

                cols[1].vertical(|ui| {
                    for pair in [100_u32, 50, 25, 0].chunks(2) {
                        ui.columns(2, |c| {
                            for (i, &pct) in pair.iter().enumerate() {
                                let sub = match pct {
                                    100 => "Full",
                                    0 => "Skipped",
                                    _ => "Partial",
                                };
                                if color_button_h(&mut c[i], ctx, ("meal_btn", pct), right_btn_h, &format!("Meal {pct}%"), sub, self.state.meal_eaten_pct == pct, card_c, t).clicked() {
                                    self.state.set_meal(pct);
                                }
                            }
                        });
                        ui.add_space(gap);
                    }

                    if color_button_h(ui, ctx, "water_btn", right_btn_h, "+ 250 mL water", &format!("{} / 1,150 mL", self.state.water_ml), false, card_c, t).clicked() {
                        self.state.water_ml += 250;
                        self.state.sweat_deficit_ml = self.state.sweat_deficit_ml.saturating_sub(250);
                    }
                    ui.add_space(gap);
                    if color_button_h(ui, ctx, "nacl_btn", right_btn_h, "+ 1g salt tablet", &format!("{} / 9 tablets", self.state.nacl_count), false, card_c, t).clicked() {
                        if self.state.nacl_count < 9 {
                            self.state.nacl_count += 1;
                        }
                    }
                });
            });
        }
    }

    fn overlay_hub(
        &mut self,
        ui: &mut Ui,
        ctx: &egui::Context,
        is_input: bool,
        card_c: CardColors,
        t: Tokens,
    ) {
        let p = self.state.phase;
        let body_h = ui.available_height();
        let gap = t.sp(8.0);

        if !is_input {
            // Left: Full-height live compass panel (read-only so tapping anywhere returns!)
            // Right: 4 metric tiles (2x2) growing cleanly to fill 100% of remaining height
            ui.columns(2, |cols| {
                let left_rect = cols[0].allocate_exact_size(Vec2::new(cols[0].available_width(), body_h), Sense::hover()).0;
                cols[0].painter().rect(left_rect, t.sp(8.0), card_c.bg, Stroke::new(t.stroke_w, card_c.border));
                let left_in = left_rect.shrink(t.sp(10.0));
                cols[0].allocate_ui_at_rect(left_in, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Surface compass").font(heading_font(t.sz(13.0))).color(t.text_main));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!("Hdg {:03.0}° · Rel {:03.0}°", self.state.compass_heading_deg, self.state.relative_bearing_deg()))
                                    .family(FontFamily::Monospace)
                                    .size(t.sz(11.0))
                                    .color(card_c.accent),
                            );
                        });
                    });

                    let hdr_used = t.sz(13.0) + t.sp(10.0);
                    let middle_h = (left_in.height() - hdr_used).max(t.sp(70.0));
                    let dial_sz = (middle_h * 0.90)
                        .min(left_in.width() * 0.88)
                        .max(t.sp(64.0));

                    ui.allocate_ui(Vec2::new(ui.available_width(), middle_h), |ui| {
                        ui.centered_and_justified(|ui| {
                            draw_working_compass(
                                ui,
                                dial_sz,
                                self.state.compass_heading_deg,
                                self.state.waypoint.bearing_deg(),
                                card_c.accent,
                                true,
                                t,
                            );
                        });
                    });
                });

                cols[1].vertical(|ui| {
                    let tile_h = ((body_h - gap) * 0.5).max(t.sp(56.0));
                    ui.columns(2, |r1| {
                        stat_card_h(&mut r1[0], tile_h, "Target distance", &format!("{} m", self.state.distance_m), card_c.accent, self.state.waypoint.label(), card_c, t);
                        stat_card_h(&mut r1[1], tile_h, "Flight phase", p.name(), t.text_main, p.hr_range(), card_c, t);
                    });
                    ui.add_space(gap);
                    ui.columns(2, |r2| {
                        stat_card_h(&mut r2[0], tile_h, "Earth signal lag", "18m 42s", t.caution, "Local autonomous", card_c, t);
                        stat_card_h(
                            &mut r2[1],
                            tile_h,
                            "Surface LAN",
                            if self.state.online { "Connected" } else { "Offline" },
                            if self.state.online { t.nominal } else { t.caution },
                            "192.168.1.10",
                            card_c,
                            t,
                        );
                    });
                });
            });
        } else {
            let avail = (body_h - gap * 3.0).max(t.sp(120.0));
            let phase_h = avail * 0.28;
            let wp_h = avail * 0.22;
            let sync_h = avail * 0.22;

            for row in 0..2 {
                ui.columns(2, |c| {
                    for col in 0..2 {
                        let i = row * 2 + col;
                        let ph = Phase::ALL[i];
                        if color_button_h(&mut c[col], ctx, ("phase_btn", i), phase_h, ph.name(), ph.hr_range(), self.state.phase == ph, card_c, t).clicked() {
                            self.state.set_phase(ph);
                        }
                    }
                });
                ui.add_space(gap);
            }

            ui.columns(2, |c| {
                if color_button_h(&mut c[0], ctx, "wp_hab_e", wp_h, "Target: Habitat", "222 m · 042°", self.state.waypoint == NavWaypoint::Habitat, card_c, t).clicked() {
                    self.state.set_waypoint(NavWaypoint::Habitat);
                }
                if color_button_h(&mut c[1], ctx, "wp_air_e", wp_h, "Target: Airlock", "145 m · 312°", self.state.waypoint == NavWaypoint::Airlock, card_c, t).clicked() {
                    self.state.set_waypoint(NavWaypoint::Airlock);
                }
            });
            ui.add_space(gap);
            if color_button_h(ui, ctx, "sync_btn", sync_h, "Force sync telemetry to hub", &format!("Flush {} queued packets", self.state.buffered_packets), false, card_c, t).clicked() {
                self.state.buffered_packets = 0;
                self.state.online = true;
            }
        }
    }

    fn overlay_tech(
        &mut self,
        ui: &mut Ui,
        ctx: &egui::Context,
        is_input: bool,
        card_c: CardColors,
        t: Tokens,
    ) {
        let body_h = ui.available_height();
        let gap = t.sp(8.0);

        if !is_input {
            let tile_h = ((body_h - gap) * 0.5).max(t.sp(56.0));
            ui.columns(2, |r1| {
                stat_card_h(&mut r1[0], tile_h, "Safety engine", "1 Hz local", t.nominal, "Zero Earth lag", card_c, t);
                stat_card_h(
                    &mut r1[1],
                    tile_h,
                    "Surface LAN",
                    if self.state.online { "Online" } else { "Offline" },
                    if self.state.online { t.nominal } else { t.caution },
                    "Auto-sync",
                    card_c,
                    t,
                );
            });
            ui.add_space(gap);
            ui.columns(2, |r2| {
                stat_card_h(&mut r2[0], tile_h, "Offline queue", &format!("{} pkts", self.state.buffered_packets), t.text_main, "Local buffer", card_c, t);
                stat_card_h(&mut r2[1], tile_h, "Display profile", &self.state.a11y.summary(), card_c.accent, "Edit via Cycle", card_c, t);
            });
        } else {
            let left_row_h = ((body_h - 3.0 * gap) / 5.0).max(t.sp(24.0));
            let slider_h = left_row_h * 2.0;
            let right_row_h = ((body_h - 4.0 * gap) / 5.0).max(t.sp(24.0));

            ui.columns(2, |cols| {
                cols[0].vertical(|ui| {
                    if let Some(v) = brightness_slider(ui, slider_h, self.state.a11y.brightness_pct, card_c, t) {
                        self.state.a11y.brightness_pct = v;
                    }
                    ui.add_space(gap);
                    for (idx, &th) in ColorTheme::ALL.iter().enumerate() {
                        if color_button_h(ui, ctx, ("theme_btn", th as u8), left_row_h, th.label(), if self.state.a11y.theme == th { "Active" } else { "" }, self.state.a11y.theme == th, card_c, t).clicked() {
                            self.state.a11y.theme = th;
                        }
                        if idx < ColorTheme::ALL.len() - 1 {
                            ui.add_space(gap);
                        }
                    }
                });

                cols[1].vertical(|ui| {
                    for pair in ColorVision::ALL.chunks(2) {
                        ui.columns(2, |c| {
                            for (i, &vis) in pair.iter().enumerate() {
                                if color_button_h(&mut c[i], ctx, ("vis_btn", vis as u8), right_row_h, vis.label(), vis.short(), self.state.a11y.vision == vis, card_c, t).clicked() {
                                    self.state.a11y.vision = vis;
                                }
                            }
                        });
                        ui.add_space(gap);
                    }

                    ui.columns(3, |c| {
                        for (i, &ts) in TextScalePreset::ALL.iter().enumerate() {
                            if color_button_h(&mut c[i], ctx, ("txt_btn", i), right_row_h, &format!("Text: {}", ts.label()), "", self.state.a11y.text_scale == ts, card_c, t).clicked() {
                                self.state.a11y.text_scale = ts;
                            }
                        }
                    });
                    ui.add_space(gap);
                    ui.columns(2, |c| {
                        if color_button_h(&mut c[0], ctx, "a11y_gl", right_row_h, "Glove+ size", if self.state.a11y.glove_targets { "ON" } else { "OFF" }, self.state.a11y.glove_targets, card_c, t).clicked() {
                            self.state.a11y.glove_targets = !self.state.a11y.glove_targets;
                        }
                        if color_button_h(&mut c[1], ctx, "a11y_rm", right_row_h, "Reduced motion", if self.state.a11y.reduced_motion { "ON" } else { "OFF" }, self.state.a11y.reduced_motion, card_c, t).clicked() {
                            self.state.a11y.reduced_motion = !self.state.a11y.reduced_motion;
                        }
                    });
                    ui.add_space(gap);
                    let lan_lbl = if self.state.online { "Test LAN blackout" } else { "Reconnect & flush" };
                    let lan_sub = if self.state.online { "Online" } else { "Offline" };
                    if color_button_h(ui, ctx, "lan_btn", right_row_h, lan_lbl, lan_sub, !self.state.online, card_c, t).clicked() {
                        self.state.online = !self.state.online;
                        if self.state.online {
                            self.state.buffered_packets = 0;
                        }
                    }
                });
            });
        }
    }

    fn overlay_hazard(
        &mut self,
        ui: &mut Ui,
        ctx: &egui::Context,
        is_input: bool,
        card_c: CardColors,
        t: Tokens,
    ) {
        let h = self.state.hazard;
        let body_h = ui.available_height();
        let gap = t.sp(8.0);

        if !is_input {
            let tile_h = ((body_h - gap) * 0.5).max(t.sp(56.0));
            ui.columns(2, |r1| {
                stat_card_h(&mut r1[0], tile_h, "Radiation", h.rad(), if h == Hazard::Radiation { t.critical } else { t.nominal }, "Limit < 2.0", card_c, t);
                stat_card_h(&mut r1[1], tile_h, "Suit CO2", h.co2(), if h == Hazard::HighCo2 { t.caution } else { t.nominal }, "Limit < 4.0", card_c, t);
            });
            ui.add_space(gap);
            ui.columns(2, |r2| {
                stat_card_h(&mut r2[0], tile_h, "Tissue oxygen", h.o2(), if h == Hazard::Hypoxia { t.critical } else { t.nominal }, "Safe >= 92%", card_c, t);
                stat_card_h(&mut r2[1], tile_h, "Surface wind", "3 m/s NE", t.text_main, "Visibility clear", card_c, t);
            });
        } else {
            let btn_h = ((body_h - gap) * 0.5).max(t.sp(48.0));
            for (row, pair) in Hazard::ALL.chunks(2).enumerate() {
                ui.columns(2, |c| {
                    for (i, hk) in pair.iter().enumerate() {
                        let sub = if *hk == Hazard::Nominal { "Clear" } else { "Trigger" };
                        if color_button_h(&mut c[i], ctx, ("haz_btn", row * 2 + i), btn_h, hk.label(), sub, self.state.hazard == *hk, card_c, t).clicked() {
                            self.state.hazard = *hk;
                        }
                    }
                });
                if row == 0 {
                    ui.add_space(gap);
                }
            }
        }
    }
}

