use eframe::egui::{self, Align, Align2, Color32, Layout, RichText, Rounding, Stroke, vec2};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use crate::app::{App, View};
use crate::net;
use shared::{MessageInfo, UserInfo};
use uuid::Uuid;

pub fn show(app: &mut App, ctx: &egui::Context, idx: usize) {
    let Some(group) = app.groups.get(idx) else {
        egui::CentralPanel::default().show(ctx, |ui| { ui.label("Gruppo non trovato"); if ui.button("Indietro").clicked() { app.nav(View::ChatList); }});
        return;
    };

    let group_id = group.group_id;
    let group_name = group.name.clone();

    // Stili
    let bg_color = ctx.style().visuals.window_fill();
    let header_bg = bg_color.linear_multiply(1.2);
    let bubble_me = Color32::from_rgb(0, 110, 210);
    let bubble_other = Color32::from_gray(55);
    let input_bg = ctx.style().visuals.extreme_bg_color;

    // --- HEADER ---
    egui::TopBottomPanel::top("chat_top_panel")
        .frame(egui::Frame::none().fill(header_bg).inner_margin(egui::Margin::symmetric(16.0, 12.0)).stroke(Stroke::new(1.0, Color32::from_white_alpha(10))))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(RichText::new("⬅").size(20.0)).frame(false)).clicked() { app.nav(View::ChatList); }
                ui.add_space(10.0);
                ui.label(RichText::new("👥").size(26.0)); // Icona bilanciata

                ui.vertical(|ui| {
                    // *** FIX 1: Dimensione intermedia (20.0) ***
                    ui.label(RichText::new(&group_name).strong().size(20.0));

                    // Sottotitolo con lista nomi
                    let subtitle = if app.current_group_members.is_empty() {
                        "Caricamento...".to_string()
                    } else {
                        generate_members_subtitle(&app.current_group_members)
                    };

                    // *** FIX 2: Dimensione intermedia (13.0) ***
                    ui.label(RichText::new(subtitle).size(13.0).color(Color32::from_gray(180)));
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    // Pulsante INFO (Toggle)
                    let info_btn_fill = if app.show_group_info { Color32::from_gray(70) } else { Color32::TRANSPARENT };
                    if ui.add(egui::Button::new(RichText::new("ℹ").size(20.0)).fill(info_btn_fill).rounding(Rounding::same(8.0))).on_hover_text("Info Gruppo & Membri").clicked() {
                        app.show_group_info = !app.show_group_info;
                        // Pulisci la ricerca se si chiude/apre per ordine
                        if !app.show_group_info {
                            app.search_query.clear();
                            app.search_results.clear();
                        }
                    }

                    if app.is_loading { ui.spinner(); }
                });
            });
        });

    // --- SIDE PANEL (INFO & SEARCH) ---
    if app.show_group_info {
        egui::SidePanel::right("group_info_panel")
            .default_width(280.0)
            .resizable(false)
            .frame(egui::Frame::none().fill(bg_color.linear_multiply(1.1)).inner_margin(16.0).stroke(Stroke::new(1.0, Color32::from_white_alpha(10))))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(RichText::new("Dettagli Gruppo").strong().size(18.0));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.add(egui::Button::new(RichText::new("×").size(28.0)).frame(false)).clicked() {
                            app.show_group_info = false;
                            app.search_query.clear();
                            app.search_results.clear();
                        }
                    });
                });
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(15.0);

                // --- SEZIONE RICERCA MEMBRI ---
                ui.label(RichText::new("Aggiungi Partecipante").strong());
                ui.add_space(5.0);

                // Input di ricerca
                let search_resp = ui.add(egui::TextEdit::singleline(&mut app.search_query)
                    .hint_text("Cerca utente...")
                    .desired_width(ui.available_width()));

                if search_resp.changed() {
                    search_users_logic(app, ctx);
                }

                // Lista Risultati Ricerca
                if !app.search_results.is_empty() {
                    ui.add_space(5.0);
                    ui.label(RichText::new("Risultati ricerca:").small().weak());

                    egui::Frame::none()
                        .fill(Color32::from_black_alpha(40))
                        .rounding(4.0)
                        .inner_margin(5.0)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical().id_source("search_res_scroll").max_height(150.0).show(ui, |ui| {
                                ui.vertical(|ui| {
                                    let results = app.search_results.clone();
                                    for user in results {
                                        let is_already_member = app.current_group_members.iter().any(|m| m.user_id == user.user_id);

                                        ui.horizontal(|ui| {
                                            ui.label("👤");
                                            ui.label(RichText::new(&user.username).strong());

                                            // Layout destra: Mostra o "Già membro" o il tasto "+"
                                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                if is_already_member {
                                                    ui.label(RichText::new("Già membro").small().italics().color(Color32::from_gray(120)));
                                                } else {
                                                    if ui.add(egui::Button::new(RichText::new("➕").color(Color32::WHITE))
                                                        .fill(Color32::from_rgb(0, 100, 200))) // Blu scuro
                                                        .on_hover_text("Aggiungi al gruppo")
                                                        .clicked()
                                                    {
                                                        app.add_member_username = user.username.clone();
                                                        add_member_logic(app, ctx, group_id);
                                                        app.search_query.clear();
                                                        app.search_results.clear();
                                                    }
                                                }
                                            });
                                        });
                                        ui.add_space(4.0);
                                        ui.separator();
                                    }
                                });
                            });
                        });
                } else if !app.search_query.trim().is_empty() {
                    ui.add_space(10.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("Nessun utente trovato").italics().color(Color32::from_gray(120)));
                    });
                }

                ui.add_space(20.0);
                ui.separator();
                ui.add_space(10.0);

                // --- LISTA MEMBRI ATTUALI ---
                ui.label(RichText::new(format!("Membri del gruppo ({})", app.current_group_members.len())).strong());
                ui.add_space(5.0);

                egui::ScrollArea::vertical().id_source("members_list").show(ui, |ui| {
                    for member in &app.current_group_members {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("👤").size(14.0));
                            if Some(member.user_id) == app.user_id {
                                ui.label(RichText::new(&member.username).strong().color(Color32::from_rgb(100, 200, 255)));
                                ui.label(RichText::new("(Tu)").small().weak());
                            } else {
                                ui.label(&member.username);
                            }
                        });
                        ui.add_space(8.0);
                    }
                });
            });
    }

    // --- TOAST NOTIFICATION ---
    if !app.status.is_empty() && !app.is_loading {
        egui::Area::new(egui::Id::new("status_overlay_group")).order(egui::Order::Foreground).anchor(Align2::CENTER_TOP, vec2(0.0, 60.0)).show(ctx, |ui| {
            egui::Frame::none().fill(Color32::from_rgb(40, 40, 40)).stroke(Stroke::new(1.0, Color32::from_gray(80))).rounding(Rounding::same(20.0)).inner_margin(egui::Margin::symmetric(20.0, 10.0)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&app.status).color(Color32::WHITE));
                    ui.add_space(10.0);
                    if ui.button(RichText::new("X").small()).clicked() { app.status.clear(); }
                });
            });
        });
    }

    // --- INPUT AREA ---
    egui::TopBottomPanel::bottom("chat_input_panel_group")
        .frame(egui::Frame::none().fill(bg_color).inner_margin(16.0))
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(input_bg)
                .rounding(Rounding::same(24.0))
                .stroke(Stroke::new(1.0, Color32::from_gray(70)))
                .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let response = ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            ui.add(egui::TextEdit::multiline(&mut app.message_input)
                                .hint_text("Scrivi al gruppo...")
                                .desired_rows(1)
                                .frame(false)
                                .desired_width((ui.available_width() - 50.0).max(100.0)))
                        }).inner;

                        let send_btn = egui::Button::new(RichText::new("➡️").size(18.0).color(Color32::WHITE))
                            .min_size(vec2(32.0, 32.0)).rounding(Rounding::same(16.0)).fill(bubble_me);

                        let clicked = ui.add_enabled(!app.message_input.trim().is_empty(), send_btn).clicked();
                        let enter_pressed = response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift);

                        if (clicked || enter_pressed) && !app.message_input.trim().is_empty() {
                            send_message(app, ctx, group_id);
                            response.request_focus();
                        }
                    });
                });
        });

    // --- MESSAGES ---
    egui::CentralPanel::default().show(ctx, |ui| {
        egui::Frame::none().fill(bg_color).inner_margin(egui::Margin::symmetric(16.0, 10.0)).show(ui, |ui| {
            egui::ScrollArea::vertical().stick_to_bottom(true).auto_shrink([false, false]).show(ui, |ui| {
                if app.current_messages.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0);
                        ui.label(RichText::new("📣").size(40.0));
                        ui.label(RichText::new("Benvenuto nel gruppo.").color(Color32::from_gray(150)));
                    });
                } else {
                    for msg in &app.current_messages {
                        draw_group_chat_bubble(ui, msg, Some(msg.sender_id) == app.user_id, bubble_me, bubble_other);
                        ui.add_space(8.0);
                    }
                }
                ui.add_space(10.0);
            });
        });
    });
}

// --- HELPERS ---

fn search_users_logic(app: &mut App, ctx: &egui::Context) {
    if let Some(token) = app.session_token {
        let query = app.search_query.clone();
        if query.trim().is_empty() {
            app.search_results.clear();
            return;
        }

        let tx = app.get_tx();
        let addr = app.server_addr.clone();
        let handle = app.rt.as_ref().unwrap().handle().clone();
        let ctx2 = ctx.clone();

        handle.spawn(async move {
            // Utilizziamo la stessa funzione di ricerca usata per le chat private
            let result = net::search_users(&addr, token, &query).await;
            let _ = tx.send(result);
            ctx2.request_repaint();
        });
    }
}

fn generate_members_subtitle(members: &[UserInfo]) -> String {
    let mut names = String::new();
    let limit = 60;

    for (i, member) in members.iter().enumerate() {
        if i > 0 {
            names.push_str(", ");
        }
        names.push_str(&member.username);

        if names.len() > limit {
            names.push_str("...");
            return names;
        }
    }
    names
}

fn draw_group_chat_bubble(ui: &mut egui::Ui, msg: &MessageInfo, is_mine: bool, color_me: Color32, color_other: Color32) {
    let max_width = 400.0;
    let layout = if is_mine { Layout::right_to_left(Align::TOP) } else { Layout::left_to_right(Align::TOP) };
    ui.allocate_ui_with_layout(vec2(ui.available_width(), 0.0), layout, |ui| {
        let rounding = if is_mine { Rounding { nw: 18.0, ne: 18.0, sw: 18.0, se: 4.0 } } else { Rounding { nw: 18.0, ne: 18.0, sw: 4.0, se: 18.0 } };
        egui::Frame::none().fill(if is_mine { color_me } else { color_other }).rounding(rounding).inner_margin(egui::Margin::symmetric(14.0, 10.0)).show(ui, |ui| {
            ui.set_max_width(max_width);
            ui.set_min_width(40.0);
            ui.vertical(|ui| {
                if !is_mine { ui.label(RichText::new(&msg.sender_username).size(11.0).strong().color(Color32::from_rgb(150, 220, 255))); }
                ui.label(RichText::new(&msg.content).size(15.0).color(Color32::WHITE));
                ui.add_space(2.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(format_timestamp(msg.sent_at)).size(10.0).color(Color32::from_white_alpha(150)));
                });
            });
        });
    });
}

fn send_message(app: &mut App, ctx: &egui::Context, group_id: uuid::Uuid) {
    if let (Some(token), Some(user_id), Some(username)) = (app.session_token, app.user_id, app.username.clone()) {
        let content = app.message_input.clone();
        if content.trim().is_empty() { return; }
        app.message_input.clear();
        let optimistic_msg = MessageInfo { message_id: Uuid::new_v4(), sender_id: user_id, sender_username: username, content: content.clone(), sent_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64 };
        app.current_messages.push(optimistic_msg);
        let tx = app.get_tx();
        let addr = app.server_addr.clone();
        let handle = app.rt.as_ref().unwrap().handle().clone();
        let ctx2 = ctx.clone();
        handle.spawn(async move {
            let result = net::send_group_message(&addr, token, group_id, &content).await;
            let _ = tx.send(result);
            ctx2.request_repaint();
        });
    }
}

fn add_member_logic(app: &mut App, ctx: &egui::Context, group_id: Uuid) {
    if let Some(token) = app.session_token {
        let tx = app.get_tx();
        app.is_loading = true;
        app.status = format!("Aggiunto {}", app.add_member_username).into();
        let addr = app.server_addr.clone();
        let username = app.add_member_username.clone();
        let handle = app.rt.as_ref().unwrap().handle().clone();
        let ctx2 = ctx.clone();
        handle.spawn(async move {
            let result = net::add_group_member(&addr, token, group_id, &username).await;
            let _ = tx.send(result);
            ctx2.request_repaint();
        });
    }
}

fn format_timestamp(unix_time: i64) -> String {
    let timestamp = UNIX_EPOCH + Duration::from_secs(unix_time as u64);
    if let Ok(elapsed) = SystemTime::now().duration_since(timestamp) {
        let secs = elapsed.as_secs();
        if secs < 60 { return "ora".to_string(); } else if secs < 3600 { return format!("{} min fa", secs / 60); } else if secs < 86400 { return format!("{} ore fa", secs / 3600); }
    }
    "pochi secondi fa".to_string()
}