use eframe::egui::{self, Align, Color32, Layout, RichText, Rounding, Stroke, vec2};
use crate::app::{App, View};
use crate::net;
use std::time::{Duration, Instant};

pub fn show(app: &mut App, ctx: &egui::Context) {
    // --- LOGICA (INVARIATA) ---

    // Autocaricamento chat e gruppi
    if !app.chats_loaded {
        if let Some(token) = app.session_token {
            app.chats_loaded = true;
            app.load_private_chats(ctx, token, true);
        }
    }

    // Logica del Timer (Debounce) per la ricerca
    if let Some(last_edit) = app.search_last_edit {
        if last_edit.elapsed() > Duration::from_millis(300) {
            app.search_last_edit = None; // Reset timer

            if let Some(token) = app.session_token {
                let query = app.search_query.trim().to_string();
                if !query.is_empty() {
                    let tx = app.get_tx();
                    let addr = app.server_addr.clone();
                    let handle = app.rt.as_ref().unwrap().handle().clone();
                    let ctx2 = ctx.clone();

                    // Spawn della ricerca
                    handle.spawn(async move {
                        let result = net::search_users(&addr, token, &query).await;
                        let _ = tx.send(result);
                        ctx2.request_repaint();
                    });
                } else {
                    app.search_results.clear();
                    app.last_search_completed = false;
                }
            }
        }
    }

    // --- STILI COMUNI (Presi da Home/Login) ---
    let panel_fill = ctx.style().visuals.window_fill();
    // Sfondo "Card": leggermente più chiaro dello sfondo finestra
    let card_fill = panel_fill.linear_multiply(1.3);
    let card_stroke = Stroke::new(1.0, Color32::from_white_alpha(15));
    let card_rounding = Rounding::same(12.0);

    let btn_blue = Color32::from_rgb(0, 110, 210);
    let btn_gray = Color32::from_rgb(50, 50, 50);

    // --- SIDE PANEL (Sinistra) ---
    egui::SidePanel::left("chat_options_panel")
        .default_width(300.0)
        .resizable(false)
        .frame(egui::Frame::none().fill(panel_fill)) // Sfondo base uniforme
        .show(ctx, |ui| {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.heading(RichText::new("Cerca & Crea").strong().size(22.0));
            });
            ui.add_space(20.0);

            // 1. CARD RICERCA
            egui::Frame::none()
                .fill(card_fill)
                .rounding(card_rounding)
                .stroke(card_stroke)
                .inner_margin(15.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🔍").size(18.0));
                        ui.label(RichText::new("Cerca Utenti").strong().size(16.0));
                    });
                    ui.add_space(10.0);

                    let response = ui.add(
                        egui::TextEdit::singleline(&mut app.search_query)
                            .hint_text("Scrivi username...")
                            .min_size(vec2(ui.available_width(), 30.0))
                    );

                    if response.changed() {
                        app.search_last_edit = Some(Instant::now());
                        if app.search_query.trim().is_empty() {
                            app.search_results.clear();
                            app.last_search_completed = false;
                        }
                    }

                    if app.search_last_edit.is_some() {
                        ui.add_space(5.0);
                        ui.spinner();
                    }

                    // Risultati Ricerca
                    if !app.search_results.is_empty() {
                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(5.0);
                        ui.label(RichText::new("Risultati:").small().weak());

                        let results = app.search_results.clone();
                        egui::ScrollArea::vertical().id_source("search_results").max_height(200.0).show(ui, |ui| {
                            for user in results {
                                ui.add_space(4.0);
                                // "Card" per singolo risultato
                                egui::Frame::none()
                                    .fill(Color32::from_black_alpha(40)) // Più scuro per contrasto
                                    .rounding(Rounding::same(8.0))
                                    .inner_margin(8.0)
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(format!("👤 {}", user.username)).strong());
                                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                let is_self = Some(user.user_id) == app.user_id;
                                                // Bottone Chat piccolo
                                                if ui.add_enabled(!is_self, egui::Button::new("💬 Chat").small()).clicked() {
                                                    // Logica avvio chat
                                                    let existing_idx = app.private_chats.iter().position(|c| c.other_user_id == user.user_id);
                                                    if let Some(idx) = existing_idx {
                                                        let chat_id = app.private_chats[idx].chat_id;
                                                        app.nav(View::PrivateChat(idx));
                                                        if let Some(token) = app.session_token {
                                                            app.load_private_messages(ctx, token, chat_id, true);
                                                        }
                                                    } else {
                                                        if let Some(token) = app.session_token {
                                                            let other_username = user.username.clone();
                                                            let tx = app.get_tx();
                                                            app.is_loading = true;
                                                            app.status = format!("Avvio chat con {}...", other_username);
                                                            let addr = app.server_addr.clone();
                                                            let handle = app.rt.as_ref().unwrap().handle().clone();
                                                            let ctx2 = ctx.clone();
                                                            handle.spawn(async move {
                                                                let result = net::start_private_chat(&addr, token, &other_username).await;
                                                                let _ = tx.send(result);
                                                                ctx2.request_repaint();
                                                            });
                                                        }
                                                    }
                                                }
                                            });
                                        });
                                    });
                            }
                        });
                    } else if !app.search_query.is_empty() && app.last_search_completed && app.search_last_edit.is_none() {
                        ui.add_space(10.0);
                        ui.label(RichText::new("Nessun utente trovato.").small().weak());
                    }
                });

            ui.add_space(20.0);

            // 2. CARD CREA GRUPPO
            egui::Frame::none()
                .fill(card_fill)
                .rounding(card_rounding)
                .stroke(card_stroke)
                .inner_margin(15.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("➕").size(18.0));
                        ui.label(RichText::new("Crea Gruppo").strong().size(16.0));
                    });
                    ui.add_space(10.0);

                    ui.add(
                        egui::TextEdit::singleline(&mut app.new_group_name)
                            .hint_text("Nome gruppo...")
                            .min_size(vec2(ui.available_width(), 30.0))
                    );

                    ui.add_space(10.0);

                    let can_create = !app.is_loading && !app.new_group_name.trim().is_empty();
                    // Bottone stile Login (Blu)
                    let create_btn = egui::Button::new(RichText::new("Crea Gruppo").strong().color(Color32::WHITE))
                        .min_size(vec2(ui.available_width(), 35.0))
                        .rounding(Rounding::same(8.0))
                        .fill(btn_blue);

                    if ui.add_enabled(can_create, create_btn).clicked() {
                        if let Some(token) = app.session_token {
                            let tx = app.get_tx();
                            app.is_loading = true;
                            app.status = format!("Creazione gruppo {}...", app.new_group_name);
                            let addr = app.server_addr.clone();
                            let name = app.new_group_name.clone();
                            let handle = app.rt.as_ref().unwrap().handle().clone();
                            let ctx2 = ctx.clone();
                            handle.spawn(async move {
                                let result = net::create_group(&addr, token, &name).await;
                                let _ = tx.send(result);
                                ctx2.request_repaint();
                            });
                        }
                    }
                });

            // *** MODIFICA: Rimosso pulsante "Aggiorna Liste" qui sotto ***
        });

    // --- CENTRAL PANEL (Liste) ---
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.add_space(20.0);

        // Intestazione
        ui.horizontal(|ui| {
            ui.heading(RichText::new("I tuoi Messaggi").size(28.0).strong());
            if app.is_loading {
                ui.add_space(10.0);
                ui.spinner();
            }
        });
        if !app.status.is_empty() {
            ui.label(RichText::new(&app.status).small().color(Color32::GRAY));
        }

        ui.add_space(25.0);

        // --- SEZIONE CHAT PRIVATE ---
        ui.label(RichText::new("👤 Chat Private").size(18.0).strong().color(Color32::from_gray(200)));
        ui.add_space(8.0);

        // Card contenitore lista
        egui::Frame::none()
            .fill(card_fill)
            .rounding(card_rounding)
            .stroke(card_stroke)
            .inner_margin(10.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().id_source("private_chats").max_height(250.0).show(ui, |ui| {
                    if app.private_chats.is_empty() {
                        ui.label(RichText::new("Nessuna chat privata avviata.").italics().weak());
                    }
                    let chats = app.private_chats.clone();
                    for (idx, chat) in chats.iter().enumerate() {
                        // Ogni chat è un bottone stile "Item"
                        let btn = egui::Button::new(
                            RichText::new(format!("👤 {}", chat.other_username)).size(16.0).strong()
                        )
                            .min_size(vec2(ui.available_width(), 40.0))
                            .rounding(Rounding::same(8.0))
                            .fill(btn_gray) // Grigio scuro per gli elementi
                            .stroke(Stroke::new(1.0, Color32::from_gray(70)));

                        if ui.add(btn).clicked() {
                            app.nav(View::PrivateChat(idx));
                            if let Some(token) = app.session_token {
                                app.load_private_messages(ctx, token, chat.chat_id, true);
                            }
                        }
                        ui.add_space(6.0);
                    }
                });
            });

        ui.add_space(30.0);

        // --- SEZIONE GRUPPI ---
        ui.label(RichText::new("👥 Gruppi").size(18.0).strong().color(Color32::from_gray(200)));
        ui.add_space(8.0);

        // Card contenitore lista gruppi
        egui::Frame::none()
            .fill(card_fill)
            .rounding(card_rounding)
            .stroke(card_stroke)
            .inner_margin(10.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().id_source("groups").max_height(ui.available_height() - 50.0).show(ui, |ui| {
                    if app.groups.is_empty() {
                        ui.label(RichText::new("Non sei in nessun gruppo.").italics().weak());
                    }
                    let groups = app.groups.clone();
                    for (idx, group) in groups.iter().enumerate() {
                        let btn = egui::Button::new(
                            RichText::new(format!("👥 {}", group.name)).size(16.0).strong()
                        )
                            .min_size(vec2(ui.available_width(), 40.0))
                            .rounding(Rounding::same(8.0))
                            .fill(btn_gray)
                            .stroke(Stroke::new(1.0, Color32::from_gray(70)));

                        if ui.add(btn).clicked() {
                            app.nav(View::GroupChat(idx));
                            if let Some(token) = app.session_token {
                                app.load_group_messages(ctx, token, group.group_id, true);
                                app.load_group_members(ctx, token, group.group_id);
                            }
                        }
                        ui.add_space(6.0);
                    }
                });
            });
    });
}