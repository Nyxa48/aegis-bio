use aegis_bio_demo::hub_data::{
    evaluate_alerts, AlertLevel, HubState, MissionPhase, QualityFlag, TelemetryPacket,
};
use eframe::egui::{
    self, Align, Color32, Frame, Layout, RichText, Rounding, ScrollArea, Stroke, Ui, Vec2,
};
use egui_plot::{Line, Plot, PlotPoints, Points};
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::thread;

const LAN_TCP_ADDR: &str = "0.0.0.0:7878";

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 840.0])
            .with_min_inner_size([960.0, 640.0])
            .with_title("AEGIS-BIO — Mission Hub Terminal (Rust)"),
        ..Default::default()
    };

    eframe::run_native(
        "AEGIS-BIO Mission Hub Terminal",
        options,
        Box::new(|cc| Box::new(HubApp::new(cc))),
    )
}

#[derive(Clone, Copy)]
struct Pal {
    bg: Color32,
    panel: Color32,
    panel_hi: Color32,
    edge: Color32,
    text: Color32,
    dim: Color32,
    accent: Color32,
    ok: Color32,
    warn: Color32,
    danger: Color32,
    bar_bg: Color32,
}

impl Default for Pal {
    fn default() -> Self {
        Self {
            bg: Color32::from_rgb(10, 16, 28),
            panel: Color32::from_rgb(22, 32, 50),
            panel_hi: Color32::from_rgb(32, 46, 70),
            edge: Color32::from_rgb(64, 88, 124),
            text: Color32::from_rgb(228, 236, 248),
            dim: Color32::from_rgb(142, 162, 188),
            accent: Color32::from_rgb(70, 190, 255),
            ok: Color32::from_rgb(72, 212, 142),
            warn: Color32::from_rgb(255, 186, 62),
            danger: Color32::from_rgb(255, 82, 82),
            bar_bg: Color32::from_rgb(12, 18, 30),
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
enum HubTab {
    Crew,
    Pantry,
    HubOnly,
}

struct HubApp {
    pal: Pal,
    tab: HubTab,
    state: HubState,
    state_path: PathBuf,
    selected_subject: Option<String>,
    selected_food_idx: usize,
    selected_ratio_idx: usize,
    auto_sim: bool,
    last_sim_tick: f64,
    meal_counter: u64,
    meal_feedback: Option<(bool, String)>,
    tcp_rx: Receiver<TelemetryPacket>,
    tcp_online: bool,
}

impl HubApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.style_mut(|s| {
            s.spacing.interact_size.y = 32.0;
            s.spacing.item_spacing = Vec2::new(8.0, 8.0);
            s.spacing.button_padding = Vec2::new(10.0, 6.0);
        });

        let state_path = PathBuf::from("data/hub_state.json");
        let nasa_dir = PathBuf::from("data/NASA");
        let mut state = HubState::load_or_seed(&state_path, Some(&nasa_dir));

        let (tx, rx) = mpsc::channel::<TelemetryPacket>();
        let ctx_clone = cc.egui_ctx.clone();
        let tcp_online = match TcpListener::bind(LAN_TCP_ADDR) {
            Ok(listener) => {
                state.push_audit(format!("[LAN] TCP JSON dinleyicisi aktif: {LAN_TCP_ADDR}"));
                thread::spawn(move || {
                    for stream in listener.incoming().flatten() {
                        let tx = tx.clone();
                        let ctx = ctx_clone.clone();
                        thread::spawn(move || {
                            let reader = BufReader::new(stream);
                            for line in reader.lines().map_while(Result::ok) {
                                let trimmed = line.trim();
                                if trimmed.is_empty() {
                                    continue;
                                }
                                if let Ok(pkt) = serde_json::from_str::<TelemetryPacket>(trimmed) {
                                    let _ = tx.send(pkt);
                                    ctx.request_repaint();
                                }
                            }
                        });
                    }
                });
                true
            }
            Err(e) => {
                state.push_audit(format!("[LAN] TCP dinleyici başlatılamadı ({LAN_TCP_ADDR}): {e}"));
                false
            }
        };

        Self {
            pal: Pal::default(),
            tab: HubTab::Crew,
            state,
            state_path,
            selected_subject: None,
            selected_food_idx: 0,
            selected_ratio_idx: 0,
            auto_sim: true,
            last_sim_tick: 0.0,
            meal_counter: 1,
            meal_feedback: None,
            tcp_rx: rx,
            tcp_online,
        }
    }

    fn apply_visuals(&self, ctx: &egui::Context) {
        let p = &self.pal;
        let mut v = egui::Visuals::dark();
        v.panel_fill = p.bg;
        v.window_fill = p.panel;
        v.extreme_bg_color = p.bar_bg;
        v.override_text_color = Some(p.text);
        v.selection.bg_fill = p.accent;
        v.selection.stroke = Stroke::new(1.0_f32, p.text);
        v.widgets.inactive.bg_fill = p.edge;
        v.widgets.inactive.weak_bg_fill = p.panel_hi;
        v.widgets.hovered.weak_bg_fill = p.edge;
        v.widgets.active.weak_bg_fill = p.accent;
        v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, p.text);
        ctx.set_visuals(v);
    }

    fn poll_tcp_packets(&mut self) {
        while let Ok(pkt) = self.tcp_rx.try_recv() {
            self.state.apply_telemetry(pkt);
        }
    }

    fn step_simulation(&mut self, t: f64) {
        self.state.sim_time = t;
        if t - self.last_sim_tick < 0.8 {
            return;
        }
        self.last_sim_tick = t;

        for (idx, astro) in self.state.crew.iter_mut().enumerate() {
            let wave = ((t * 1.3 + idx as f64 * 1.7).sin() * 2.4
                + (t * 2.9 + idx as f64).cos() * 1.1) as f32;
            let base = match astro.mission_phase {
                MissionPhase::EvaSpacewalk => 126.0,
                MissionPhase::LaunchAscent => 114.0,
                MissionPhase::ReentryLanding => 106.0,
                MissionPhase::OrbitIva => 72.0 + (idx as f32 * 3.0),
            };
            let panic_boost = if astro.panic { 38.0 } else { 0.0 };
            astro.bpm = (base + wave + panic_boost).clamp(40.0, 200.0);
            astro.push_hr_sample(t);
        }
    }

    fn bump_crew_pulse(&mut self) {
        let t = self.state.sim_time;
        for (idx, astro) in self.state.crew.iter_mut().enumerate() {
            let delta = if idx % 2 == 0 { 6.0 } else { -4.0 };
            astro.bpm = (astro.bpm + delta).clamp(45.0, 185.0);
            astro.push_hr_sample(t + 0.1);
        }
        self.state.push_audit("[SIM] Manuel nabız dalgası uygulandı.");
    }
}

impl eframe::App for HubApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_visuals(ctx);
        self.poll_tcp_packets();

        let t = ctx.input(|i| i.time);
        if self.auto_sim {
            self.step_simulation(t);
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }

        // Top Bar
        egui::TopBottomPanel::top("hub_header")
            .frame(Frame::none().fill(self.pal.panel).inner_margin(egui::Margin::symmetric(14.0, 10.0)))
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("AEGIS-BIO // MISSION HUB TERMINAL")
                            .size(18.0)
                            .strong()
                            .color(self.pal.accent),
                    );
                    ui.separator();

                    let tab_btn = |ui: &mut Ui, active: bool, text: &str, fill: Color32| {
                        let btn = egui::Button::new(
                            RichText::new(text)
                                .strong()
                                .color(if active { Color32::BLACK } else { self.pal.text }),
                        )
                        .fill(if active { fill } else { self.pal.panel_hi });
                        ui.add(btn)
                    };

                    if tab_btn(ui, self.tab == HubTab::Crew, "Mürettebat (Canlı Nabız)", self.pal.accent).clicked() {
                        self.tab = HubTab::Crew;
                    }
                    if tab_btn(ui, self.tab == HubTab::Pantry, "Yemek Deposu (ISS)", self.pal.accent).clicked() {
                        self.tab = HubTab::Pantry;
                    }
                    if tab_btn(ui, self.tab == HubTab::HubOnly, "🔒 Hub-Only (Radyasyon & Gizli)", self.pal.danger).clicked() {
                        self.tab = HubTab::HubOnly;
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Nabız Simüle Et").clicked() {
                            self.bump_crew_pulse();
                        }
                        ui.checkbox(&mut self.auto_sim, "Oto-Akış");

                        let tcp_text = if self.tcp_online {
                            format!("LAN TCP 7878 • {} pkt", self.state.packets_received)
                        } else {
                            "LAN TCP Çevrimdışı".into()
                        };
                        let tcp_col = if self.tcp_online { self.pal.ok } else { self.pal.danger };
                        badge(ui, &tcp_text, tcp_col, self.pal.bar_bg);
                        badge(ui, "MARS DTN GECİKME: 20d 00s", self.pal.warn, self.pal.bar_bg);
                    });
                });
            });

        // Main Body
        egui::CentralPanel::default()
            .frame(Frame::none().fill(self.pal.bg).inner_margin(egui::Margin::same(14.0)))
            .show(ctx, |ui| {
                ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match self.tab {
                    HubTab::Crew => self.render_crew_tab(ui),
                    HubTab::Pantry => self.render_pantry_tab(ui),
                    HubTab::HubOnly => self.render_hub_only_tab(ui),
                });
            });
    }
}

impl HubApp {
    fn render_crew_tab(&mut self, ui: &mut Ui) {
        let p = self.pal;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("MÜRETTEBAT DURUMU — PROFİL DETAYI İÇİN KARTA TIKLAYIN")
                    .size(12.0)
                    .strong()
                    .color(p.dim),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new("Anlık nabız değerleri profil açılmadan canlı izlenir")
                        .size(11.5)
                        .color(p.dim),
                );
            });
        });
        ui.add_space(6.0);

        // 4 Crew Cards (overview without clicking profile)
        let crew_len = self.state.crew.len().max(1);
        let mut clicked_id: Option<String> = None;
        ui.columns(crew_len, |cols| {
            for (idx, astro) in self.state.crew.iter().enumerate() {
                let ui = &mut cols[idx];
                let alerts = evaluate_alerts(astro);
                let is_selected = self.selected_subject.as_deref() == Some(astro.subject_id.as_str());
                let has_crit = alerts.iter().any(|a| a.level == AlertLevel::Critical);
                let has_warn = alerts.iter().any(|a| a.level == AlertLevel::Warning);

                let stroke_col = if has_crit {
                    p.danger
                } else if has_warn {
                    p.warn
                } else if is_selected {
                    p.accent
                } else {
                    p.edge
                };

                let frame_resp = Frame::none()
                    .fill(if is_selected { p.panel_hi } else { p.panel })
                    .stroke(Stroke::new(if is_selected || has_crit { 2.0_f32 } else { 1.0_f32 }, stroke_col))
                    .rounding(Rounding::same(12.0))
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            badge(ui, &astro.subject_id, p.accent, p.bar_bg);
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(RichText::new(astro.mission_phase.label()).size(11.0).monospace().color(p.dim));
                            });
                        });
                        ui.add_space(4.0);
                        ui.label(RichText::new(&astro.name).size(15.0).strong().color(p.text));
                        ui.label(RichText::new(&astro.role).size(11.5).color(p.dim));
                        ui.add_space(8.0);

                        // Live BPM box
                        Frame::none()
                            .fill(p.bar_bg)
                            .stroke(Stroke::new(1.0_f32, p.edge))
                            .rounding(Rounding::same(8.0))
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.label(RichText::new("ANLIK NABIZ").size(10.5).color(p.dim));
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(format!("{:.0}", astro.bpm))
                                                    .size(28.0)
                                                    .strong()
                                                    .color(p.danger),
                                            );
                                            ui.label(RichText::new("BPM").size(12.0).color(p.dim));
                                        });
                                    });
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.vertical(|ui| {
                                            ui.label(RichText::new(format!("O2: %{:.1}", astro.oxygen)).size(12.0).monospace().color(p.accent));
                                            ui.label(RichText::new(format!("BP: {:.0} mmHg", astro.pressure)).size(12.0).monospace().color(p.text));
                                        });
                                    });
                                });
                            });

                        ui.add_space(6.0);
                        if alerts.is_empty() {
                            ui.label(RichText::new("● Tüm sistemler nominal").size(11.5).color(p.ok));
                        } else {
                            for al in alerts {
                                let col = if al.level == AlertLevel::Critical { p.danger } else { p.warn };
                                ui.label(RichText::new(format!("⚠ {}", al.message)).size(11.5).strong().color(col));
                            }
                        }

                        ui.add_space(4.0);
                        if ui.button(if is_selected { "Profil Açık ✓" } else { "Profili Aç ›" }).clicked() {
                            clicked_id = Some(astro.subject_id.clone());
                        }
                    });

                if frame_resp.response.interact(egui::Sense::click()).clicked() {
                    clicked_id = Some(astro.subject_id.clone());
                }
            }
        });

        if let Some(id) = clicked_id {
            if self.selected_subject.as_deref() != Some(id.as_str()) {
                self.selected_subject = Some(id);
                self.meal_feedback = None;
            }
        }

        ui.add_space(14.0);

        // Detailed Profile Panel when an astronaut is selected
        if let Some(subject_id) = self.selected_subject.clone() {
            self.render_profile_detail(ui, &subject_id);
        }
    }

    fn render_profile_detail(&mut self, ui: &mut Ui, subject_id: &str) {
        let p = self.pal;
        let Some(astro_idx) = self.state.crew.iter().position(|a| a.subject_id == subject_id) else {
            return;
        };

        let mut close_requested = false;
        let mut new_phase: Option<MissionPhase> = None;
        let mut clear_panic = false;
        let mut log_meal_req: Option<(String, f32)> = None;

        Frame::none()
            .fill(p.panel)
            .stroke(Stroke::new(1.5_f32, p.accent))
            .rounding(Rounding::same(14.0))
            .inner_margin(16.0)
            .show(ui, |ui| {
                let astro = &self.state.crew[astro_idx];

                // Profile Header
                ui.horizontal_wrapped(|ui| {
                    badge(ui, &astro.subject_id, p.accent, p.bar_bg);
                    ui.label(RichText::new(&astro.name).size(19.0).strong().color(p.text));
                    ui.label(RichText::new(format!("• {}", astro.role)).size(13.0).color(p.dim));

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Kapat ✕").clicked() {
                            close_requested = true;
                        }
                        if astro.panic && ui.add(egui::Button::new("Panik Alarmını Sıfırla").fill(p.danger)).clicked() {
                            clear_panic = true;
                        }
                        for phase in MissionPhase::ALL.iter().rev() {
                            let active = astro.mission_phase == *phase;
                            let btn = egui::Button::new(
                                RichText::new(phase.label())
                                    .size(11.0)
                                    .color(if active { Color32::BLACK } else { p.text }),
                            )
                            .fill(if active { p.accent } else { p.bar_bg });
                            if ui.add(btn).clicked() {
                                new_phase = Some(*phase);
                            }
                        }
                    });
                });

                ui.separator();
                ui.add_space(6.0);

                // Two-column layout: Left (Vitals + HR Plot), Right (Nutrition + Meal Log)
                ui.columns(2, |cols| {
                    // Left: Biometrics & Plot
                    {
                        let ui = &mut cols[0];
                        let astro = &self.state.crew[astro_idx];
                        card(ui, &p, "ANLIK BİYOMETRİ & FAZ EŞİĞİ", |ui| {
                            ui.columns(3, |c| {
                                stat_cell(&mut c[0], &p, "Nabız", &format!("{:.0} BPM", astro.bpm), p.danger);
                                stat_cell(&mut c[1], &p, "Oksijen", &format!("%{:.1}", astro.oxygen), p.accent);
                                stat_cell(&mut c[2], &p, "Basınç", &format!("{:.0} mmHg", astro.pressure), p.text);
                            });
                            ui.add_space(4.0);
                            ui.columns(3, |c| {
                                stat_cell(&mut c[0], &p, "Nitrojen", &format!("%{:.1}", astro.nitrogen), p.text);
                                stat_cell(&mut c[1], &p, "Hemoglobin", &format!("{:.1} g/dL", astro.hemoglobin), p.ok);
                                stat_cell(&mut c[2], &p, "Sıcaklık", &format!("{:.1} °C", astro.temperature), p.text);
                            });
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new(format!(
                                    "Aktif Faz: {} • Nabız Alarm Eşiği: >{:.0} BPM • Sodyum Limiti: {:.0} mg/gün",
                                    astro.mission_phase.label(),
                                    astro.mission_phase.hr_limit(),
                                    astro.daily_sodium_cap_mg
                                ))
                                .size(11.5)
                                .color(p.dim),
                            );
                        });

                        ui.add_space(8.0);
                        card(ui, &p, "CANLI NABIZ ZAMAN ÇİZELGESİ", |ui| {
                            let pts: PlotPoints = astro.hr_history.iter().copied().collect();
                            Plot::new("hub_hr_plot")
                                .height(150.0)
                                .allow_zoom(false)
                                .allow_drag(false)
                                .allow_scroll(false)
                                .include_y(50.0)
                                .include_y(165.0)
                                .show(ui, |pu| {
                                    pu.line(Line::new(pts).color(p.accent).width(2.0_f32));
                                });
                        });
                    }

                    // Right: Nutrition Budget, Meal Log, Ration Recommendations
                    {
                        let ui = &mut cols[1];
                        let advice = self.state.recommend_rations(subject_id);
                        let astro = &self.state.crew[astro_idx];

                        card(ui, &p, "GÜNLÜK BESİN & SODYUM BÜTÇESİ", |ui| {
                            ui.columns(3, |c| {
                                stat_cell(
                                    &mut c[0],
                                    &p,
                                    "Kalan Enerji",
                                    &format!("{:.0} kcal", advice.remaining_kcal),
                                    p.ok,
                                );
                                stat_cell(
                                    &mut c[1],
                                    &p,
                                    "Kalan Protein",
                                    &format!("{:.0} g", advice.remaining_protein),
                                    p.accent,
                                );
                                stat_cell(
                                    &mut c[2],
                                    &p,
                                    "Kalan Sodyum",
                                    &format!("{:.0} mg", advice.remaining_sodium_mg),
                                    p.warn,
                                );
                            });
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(format!(
                                    "Tüketilen: {:.0}/{:.0} kcal • {:.0}/{:.0} g protein • {:.0}/{:.0} mg Na",
                                    astro.kcal_consumed,
                                    astro.daily_kcal_target,
                                    astro.protein_consumed,
                                    astro.daily_protein_target,
                                    astro.sodium_consumed_mg,
                                    astro.daily_sodium_cap_mg
                                ))
                                .size(11.5)
                                .color(p.dim),
                            );
                        });

                        ui.add_space(8.0);
                        card(ui, &p, "ÖĞÜN KAYDI (TÜKETİLEN ORAN)", |ui| {
                            if self.selected_food_idx >= self.state.pantry.len() {
                                self.selected_food_idx = 0;
                            }
                            let selected_food_name = self
                                .state
                                .pantry
                                .get(self.selected_food_idx)
                                .map(|f| format!("{} ({:.0} kcal, stok: {})", f.name, f.kcal, f.stock_count))
                                .unwrap_or_else(|| "Paket yok".into());

                            egui::ComboBox::from_id_source("meal_food_combo")
                                .selected_text(selected_food_name)
                                .width(ui.available_width() - 12.0)
                                .show_ui(ui, |ui| {
                                    for (i, item) in self.state.pantry.iter().enumerate() {
                                        let label = format!(
                                            "{} • {:.0} kcal • {:.0}mg Na (Stok: {})",
                                            item.name, item.kcal, item.sodium_mg, item.stock_count
                                        );
                                        ui.selectable_value(&mut self.selected_food_idx, i, label);
                                    }
                                });

                            ui.add_space(4.0);
                            let ratios = [(1.0_f32, "%100"), (0.5_f32, "%50"), (0.25_f32, "%25"), (0.0_f32, "%0")];
                            ui.horizontal(|ui| {
                                for (i, (_, lbl)) in ratios.iter().enumerate() {
                                    ui.selectable_value(&mut self.selected_ratio_idx, i, *lbl);
                                }
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui.add(egui::Button::new("Öğün Kaydet").fill(p.ok)).clicked() {
                                        if let Some(food) = self.state.pantry.get(self.selected_food_idx) {
                                            log_meal_req = Some((food.id.clone(), ratios[self.selected_ratio_idx].0));
                                        }
                                    }
                                });
                            });

                            if let Some((ok, ref msg)) = self.meal_feedback {
                                ui.label(RichText::new(msg).size(12.0).color(if ok { p.ok } else { p.danger }));
                            }
                        });

                        ui.add_space(8.0);
                        card(ui, &p, "UYGUN SIRADAKİ ISS PAKETLERİ (SODYUM BÜTÇESİNE GÖRE)", |ui| {
                            if advice.recommendations.is_empty() {
                                ui.label(
                                    RichText::new("Sodyum bütçesini veya stoku aşmayan uygun paket bulunamadı.")
                                        .size(12.0)
                                        .color(p.danger),
                                );
                            } else {
                                for rec in &advice.recommendations {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(&rec.name).size(12.5).strong().color(p.text));
                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            ui.label(
                                                RichText::new(format!(
                                                    "{:.0} kcal • {:.0}g P • {:.0}mg Na • Stok: {}",
                                                    rec.kcal, rec.protein_g, rec.sodium_mg, rec.stock_count
                                                ))
                                                .size(11.5)
                                                .monospace()
                                                .color(p.dim),
                                            );
                                        });
                                    });
                                }
                            }
                        });
                    }
                });

                ui.add_space(10.0);

                // Historical Lab Table (OSD-569 / OSD-575)
                let meas = self.state.measurements_for(subject_id);
                card(ui, &p, "TARİHSEL KAN & METABOLİZMA GEÇMİŞİ (NASA OSD-569 / OSD-575)", |ui| {
                    ui.label(
                        RichText::new("Olay ve gün ofsetine (L-3 uçuş öncesi, R+1 dönüş sonrası) göre sıralı • Boş değerler sıfıra çevrilmez")
                            .size(11.5)
                            .color(p.dim),
                    );
                    ui.add_space(4.0);
                    egui::Grid::new("meas_grid")
                        .num_columns(7)
                        .striped(true)
                        .spacing([14.0, 6.0])
                        .show(ui, |ui| {
                            for h in ["Örnek", "Zaman", "Parametre", "Değer", "Birim", "Kaynak", "Kalite"] {
                                ui.label(RichText::new(h).size(11.5).strong().color(p.dim));
                            }
                            ui.end_row();

                            for m in meas {
                                let time_tag = if m.day_offset >= 0 {
                                    format!("{} (R+{})", m.event, m.day_offset)
                                } else {
                                    format!("{} (L{})", m.event, m.day_offset)
                                };
                                let val_str = m.value.map_or_else(|| "—".into(), |v| format!("{v:.2}"));
                                let q_col = match m.quality_flag {
                                    QualityFlag::Ok => p.ok,
                                    QualityFlag::Missing => p.dim,
                                    QualityFlag::Review => p.warn,
                                };

                                ui.label(RichText::new(&m.sample_id).size(12.0).monospace());
                                ui.label(RichText::new(time_tag).size(12.0).monospace());
                                ui.label(RichText::new(&m.metric).size(12.0));
                                ui.label(RichText::new(val_str).size(12.0).monospace().strong());
                                ui.label(RichText::new(&m.unit).size(12.0).color(p.dim));
                                ui.label(RichText::new(&m.source).size(12.0).color(p.dim));
                                ui.label(RichText::new(m.quality_flag.label()).size(12.0).monospace().color(q_col));
                                ui.end_row();
                            }
                        });
                });
            });

        if close_requested {
            self.selected_subject = None;
        }
        if let Some(phase) = new_phase {
            let astro = &mut self.state.crew[astro_idx];
            astro.mission_phase = phase;
            astro.daily_sodium_cap_mg = phase.sodium_cap_mg();
            self.state.push_audit(format!("[FAZ] {} -> {}", subject_id, phase.label()));
            self.state.save(&self.state_path);
        }
        if clear_panic {
            self.state.crew[astro_idx].panic = false;
            self.state.push_audit(format!("[ALARM] {} panik durumu Hub tarafından sıfırlandı.", subject_id));
            self.state.save(&self.state_path);
        }
        if let Some((food_id, ratio)) = log_meal_req {
            let eid = format!("hub-meal-{}-{}", self.meal_counter, self.state.sim_time as u64);
            self.meal_counter += 1;
            match self.state.log_meal(&eid, subject_id, &food_id, ratio) {
                Ok(_) => {
                    self.meal_feedback = Some((true, "Öğün kaydedildi ve stok güncellendi.".into()));
                    self.state.save(&self.state_path);
                }
                Err(err) => {
                    self.meal_feedback = Some((false, err));
                }
            }
        }
    }

    fn render_pantry_tab(&mut self, ui: &mut Ui) {
        let p = self.pal;
        ui.horizontal(|ui| {
            ui.label(RichText::new("YEMEK DEPOSU ENVANTERİ (SİMÜLE ISS KATALOĞU)").size(13.0).strong().color(p.dim));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new("Öğün kaydı yapıldığında stok otomatik düşer").size(11.5).color(p.dim));
            });
        });
        ui.add_space(6.0);

        let mut restock_idx: Option<usize> = None;
        card(ui, &p, "AKTİF ISS RASYON STOKLARI", |ui| {
            egui::Grid::new("pantry_grid")
                .num_columns(9)
                .striped(true)
                .spacing([16.0, 8.0])
                .show(ui, |ui| {
                    for h in ["Kod", "Ürün Adı", "Kategori", "Porsiyon", "Enerji", "Protein", "Sodyum", "Stok", "İşlem"] {
                        ui.label(RichText::new(h).size(12.0).strong().color(p.dim));
                    }
                    ui.end_row();

                    for (i, item) in self.state.pantry.iter().enumerate() {
                        let stock_col = if item.stock_count < 5 { p.warn } else { p.ok };
                        ui.label(RichText::new(&item.id).monospace().color(p.accent));
                        ui.label(RichText::new(&item.name).strong());
                        ui.label(RichText::new(&item.category).color(p.dim));
                        ui.label(RichText::new(&item.serving_size).color(p.dim));
                        ui.label(RichText::new(format!("{:.0} kcal", item.kcal)).monospace());
                        ui.label(RichText::new(format!("{:.0} g", item.protein_g)).monospace());
                        ui.label(RichText::new(format!("{:.0} mg", item.sodium_mg)).monospace());
                        ui.label(RichText::new(format!("{}", item.stock_count)).monospace().strong().color(stock_col));
                        if ui.button("+1 Stok").clicked() {
                            restock_idx = Some(i);
                        }
                        ui.end_row();
                    }
                });
        });

        if let Some(i) = restock_idx {
            self.state.pantry[i].stock_count += 1;
            self.state.save(&self.state_path);
        }
    }

    fn render_hub_only_tab(&mut self, ui: &mut Ui) {
        let p = self.pal;
        Frame::none()
            .fill(Color32::from_rgb(48, 16, 26))
            .stroke(Stroke::new(1.5_f32, p.danger))
            .rounding(Rounding::same(12.0))
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("🔒 HUB-ONLY // ASTRONOT TERMİNALİNE KAPALI VERİLER")
                            .size(14.0)
                            .strong()
                            .color(p.danger),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        badge(ui, "SCOPE: FLIGHT SURGEON ONLY", p.danger, p.bar_bg);
                    });
                });
                ui.label(
                    RichText::new("Tarihsel Liulin-5-D3 kesiti (2013-01-01, 19–318 sn aralıklı). 1 Hz canlı akış değildir ve doğrudan insan organ dozu değildir.")
                        .size(12.0)
                        .color(p.text),
                );
            });

        ui.add_space(10.0);

        // Radiation statistics & chart
        let min_dose = self
            .state
            .radiation_readings
            .iter()
            .map(|r| r.absorbed_dose_rate)
            .fold(f64::INFINITY, f64::min);
        let max_dose = self
            .state
            .radiation_readings
            .iter()
            .map(|r| r.absorbed_dose_rate)
            .fold(0.0_f64, f64::max);

        card(ui, &p, "RADYASYON DOZ HIZI GRAFİĞİ (LIULIN-5-D3 • µGy/saat)", |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Kayıt: {}", self.state.radiation_readings.len())).monospace().color(p.dim));
                ui.separator();
                ui.label(RichText::new(format!("Min: {:.4} µGy/saat", min_dose)).monospace().color(p.ok));
                ui.separator();
                ui.label(RichText::new(format!("Maks: {:.4} µGy/saat", max_dose)).monospace().color(p.danger));
            });
            ui.add_space(4.0);

            let pts: PlotPoints = self
                .state
                .radiation_readings
                .iter()
                .map(|r| [r.elapsed_min, r.absorbed_dose_rate])
                .collect();
            let marker_pts: PlotPoints = self
                .state
                .radiation_readings
                .iter()
                .map(|r| [r.elapsed_min, r.absorbed_dose_rate])
                .collect();

            Plot::new("hub_rad_plot")
                .height(220.0)
                .show_axes([true, true])
                .x_axis_label("Kesit Süresi (dk)")
                .y_axis_label("µGy/saat")
                .show(ui, |pu| {
                    pu.line(Line::new(pts).color(p.danger).width(2.0_f32));
                    pu.points(Points::new(marker_pts).color(p.warn).radius(4.0_f32));
                });
        });

        ui.add_space(10.0);

        ui.columns(2, |cols| {
            // Left: Radiation sample log
            card(&mut cols[0], &p, "SON RADYASYON KESİT KAYITLARI", |ui| {
                ScrollArea::vertical().max_height(190.0).show(ui, |ui| {
                    egui::Grid::new("rad_grid")
                        .num_columns(4)
                        .striped(true)
                        .spacing([12.0, 4.0])
                        .show(ui, |ui| {
                            for h in ["Zaman (UTC)", "Cihaz", "Doz Hızı", "Kaynak"] {
                                ui.label(RichText::new(h).size(11.5).strong().color(p.dim));
                            }
                            ui.end_row();
                            for r in self.state.radiation_readings.iter().take(25) {
                                ui.label(RichText::new(&r.timestamp_raw).size(11.5).monospace());
                                ui.label(RichText::new(&r.instrument_id).size(11.5));
                                ui.label(RichText::new(format!("{:.4} {}", r.absorbed_dose_rate, r.unit)).size(11.5).monospace().color(p.danger));
                                ui.label(RichText::new(&r.source_file).size(11.5).color(p.dim));
                                ui.end_row();
                            }
                        });
                });
            });

            // Right: Audit / DTN Log
            card(&mut cols[1], &p, "GÖREV KONTROL DENETİM & LAN DTN GÜNLÜĞÜ", |ui| {
                ScrollArea::vertical()
                    .max_height(190.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for log in &self.state.audit_logs {
                            let col = if log.contains("ALARM") || log.contains("UYARI") {
                                p.danger
                            } else if log.contains("DTN") {
                                p.warn
                            } else {
                                p.text
                            };
                            ui.label(RichText::new(log).monospace().size(11.5).color(col));
                        }
                    });
            });
        });
    }
}

fn card(ui: &mut Ui, p: &Pal, title: &str, add: impl FnOnce(&mut Ui)) {
    Frame::none()
        .fill(p.panel_hi)
        .stroke(Stroke::new(1.0_f32, p.edge))
        .rounding(Rounding::same(10.0))
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).size(11.5).strong().color(p.dim));
            ui.add_space(4.0);
            add(ui);
        });
}

fn stat_cell(ui: &mut Ui, p: &Pal, label: &str, value: &str, col: Color32) {
    Frame::none()
        .fill(p.bar_bg)
        .rounding(Rounding::same(6.0))
        .inner_margin(6.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(label).size(10.5).color(p.dim));
            ui.label(RichText::new(value).size(14.5).strong().monospace().color(col));
        });
}

fn badge(ui: &mut Ui, text: &str, fg: Color32, bg: Color32) {
    Frame::none()
        .fill(bg)
        .stroke(Stroke::new(1.0_f32, fg))
        .rounding(Rounding::same(6.0))
        .inner_margin(egui::Margin::symmetric(8.0, 3.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.0).monospace().strong().color(fg));
        });
}
