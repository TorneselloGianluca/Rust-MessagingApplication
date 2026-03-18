use eframe::egui::{self, Align, Color32, Layout, RichText, Rounding, Stroke, vec2};
use crate::app::{App, View};
use crate::net;
use shared::MessageInfo;
use uuid::Uuid;

pub fn show(app: &mut App, ctx: &egui::Context, idx: usize) {
    let Some(chat) = app.private_chats.get(idx) else {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.label("Chat non trovata");
                if ui.button("← Torna alla lista").clicked() { app.nav(View::ChatList); }
            });
        });
        return;
    };

    let chat_id = chat.chat_id;
    let other_username = chat.other_username.clone();

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
                if ui.add(egui::Button::new(RichText::new("⬅").size(18.0)).frame(false)).clicked() { app.nav(View::ChatList); }
                ui.add_space(10.0);
                ui.label(RichText::new("👤").size(24.0));

                // Solo nome utente, centrato
                ui.label(RichText::new(&other_username).strong().size(18.0));

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if app.is_loading { ui.spinner(); }
                });
            });
        });

    // --- INPUT AREA ---
    egui::TopBottomPanel::bottom("chat_input_panel")
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
                                .hint_text("Scrivi un messaggio...")
                                .desired_rows(1)
                                .frame(false)
                                .desired_width((ui.available_width() - 50.0).max(100.0)))
                        }).inner;

                        // *** FIX: Usiamo "➡️" (Emoji) invece di caratteri unicode complessi ***
                        let send_btn = egui::Button::new(RichText::new("➡️").size(18.0).color(Color32::WHITE))
                            .min_size(vec2(32.0, 32.0)).rounding(Rounding::same(16.0)).fill(bubble_me);

                        let clicked = ui.add_enabled(!app.message_input.trim().is_empty(), send_btn).clicked();
                        let enter_pressed = response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift);

                        if (clicked || enter_pressed) && !app.message_input.trim().is_empty() {
                            send_message(app, ctx, chat_id);
                            response.request_focus();
                        }
                    });
                });
        });

    // --- MESSAGGI ---
    egui::CentralPanel::default().show(ctx, |ui| {
        egui::Frame::none().fill(bg_color).inner_margin(egui::Margin::symmetric(16.0, 10.0)).show(ui, |ui| {
            egui::ScrollArea::vertical().stick_to_bottom(true).auto_shrink([false, false]).show(ui, |ui| {
                if app.current_messages.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0);
                        ui.label(RichText::new("👋").size(40.0));
                        ui.label(RichText::new("Nessun messaggio.").color(Color32::from_gray(150)));
                    });
                } else {
                    for msg in &app.current_messages {
                        let is_mine = Some(msg.sender_id) == app.user_id;
                        draw_chat_bubble(ui, msg, is_mine, bubble_me, bubble_other);
                        ui.add_space(8.0);
                    }
                }
                ui.add_space(10.0);
            });
        });
    });
}

// --- UTILS (Invariati) ---
fn draw_chat_bubble(ui: &mut egui::Ui, msg: &MessageInfo, is_mine: bool, color_me: Color32, color_other: Color32) {
    let max_width = 400.0;
    let layout = if is_mine { Layout::right_to_left(Align::TOP) } else { Layout::left_to_right(Align::TOP) };
    ui.allocate_ui_with_layout(vec2(ui.available_width(), 0.0), layout, |ui| {
        let rounding = if is_mine { Rounding { nw: 18.0, ne: 18.0, sw: 18.0, se: 4.0 } } else { Rounding { nw: 18.0, ne: 18.0, sw: 4.0, se: 18.0 } };
        egui::Frame::none().fill(if is_mine { color_me } else { color_other }).rounding(rounding).inner_margin(egui::Margin::symmetric(14.0, 10.0)).show(ui, |ui| {
            ui.set_max_width(max_width);
            ui.set_min_width(40.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(&msg.content).size(15.0).color(Color32::WHITE));
                ui.add_space(2.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(format_timestamp(msg.sent_at)).size(10.0).color(if is_mine { Color32::from_white_alpha(180) } else { Color32::from_gray(160) }));
                });
            });
        });
    });
}

fn send_message(app: &mut App, ctx: &egui::Context, chat_id: uuid::Uuid) {
    if let (Some(token), Some(user_id), Some(username)) = (app.session_token, app.user_id, app.username.clone()) {
        let content = app.message_input.clone();
        if content.trim().is_empty() { return; }
        app.message_input.clear();
        let optimistic_msg = MessageInfo { message_id: Uuid::new_v4(), sender_id: user_id, sender_username: username, content: content.clone(), sent_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64 };
        app.current_messages.push(optimistic_msg);
        let tx = app.get_tx();
        let addr = app.server_addr.clone();
        let handle = app.rt.as_ref().unwrap().handle().clone();
        let ctx2 = ctx.clone();
        handle.spawn(async move {
            let result = net::send_private_message(&addr, token, chat_id, &content).await;
            let _ = tx.send(result);
            ctx2.request_repaint();
        });
    }
}

fn format_timestamp(unix_time: i64) -> String {
    use std::time::{SystemTime, UNIX_EPOCH, Duration};
    let timestamp = UNIX_EPOCH + Duration::from_secs(unix_time as u64);
    if let Ok(elapsed) = SystemTime::now().duration_since(timestamp) {
        let secs = elapsed.as_secs();
        if secs < 60 { return "ora".to_string(); }
        else if secs < 3600 { return format!("{} min fa", secs / 60); }
        else if secs < 86400 { return format!("{} ore fa", secs / 3600); }
    }
    "pochi secondi fa".to_string()
}