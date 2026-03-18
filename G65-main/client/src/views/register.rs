use eframe::egui::{
    self, Align, Color32, Layout, RichText, Rounding, Stroke, vec2,
};
use crate::app::{App, View};
use crate::net;

pub fn show(app: &mut App, ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.centered_and_justified(|ui| {
        egui::Frame::none()
            .fill(ui.style().visuals.window_fill().linear_multiply(1.3))
            .rounding(Rounding::same(20.0))
            .stroke(Stroke::new(1.0, Color32::from_white_alpha(15)))
            .inner_margin(40.0)
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {

                    // --- ICONA REGISTRAZIONE ---
                    ui.label(
                        RichText::new("📝")
                            .size(60.0)
                    );

                    ui.add_space(10.0);

                    // --- TITOLO ---
                    ui.heading(
                        RichText::new("Registrati")
                            .size(34.0)
                            .strong()
                            .color(Color32::from_gray(240)),
                    );

                    ui.add_space(8.0);

                    // --- SOTTOTITOLO ---
                    ui.label(
                        RichText::new("Crea un nuovo account su Ruggine.")
                            .size(15.0)
                            .color(Color32::from_gray(170))
                            .italics(),
                    );

                    ui.add_space(30.0);

                    // --- FORM CENTRATO ---
                    ui.vertical_centered(|ui| {
                        ui.set_max_width(420.0);

                        ui.with_layout(Layout::top_down(Align::Min), |ui| {
                            let input_height = 40.0;

                            // USERNAME
                            ui.label(
                                RichText::new("Username")
                                    .size(14.0)
                                    .color(Color32::from_gray(210)),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut app.reg_username)
                                    .hint_text("Scegli un username")
                                    .min_size(vec2(ui.available_width(), input_height)),
                            );

                            ui.add_space(16.0);

                            // PASSWORD
                            ui.label(
                                RichText::new("Password")
                                    .size(14.0)
                                    .color(Color32::from_gray(210)),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut app.reg_password)
                                    .password(true)
                                    .hint_text("Scegli una password")
                                    .min_size(vec2(ui.available_width(), input_height)),
                            );

                            ui.add_space(30.0);

                            // --- PULSANTI COME NELLA HOME ---
                            let btn_width = 130.0;
                            let btn_height = 45.0;
                            let btn_spacing = 20.0;
                            let total_width = 2.0 * btn_width + btn_spacing;

                            ui.horizontal(|ui| {
                                let available = ui.available_width();
                                let left_pad = ((available - total_width).max(0.0)) / 2.0;
                                ui.add_space(left_pad);

                                // Indietro
                                let back_btn = egui::Button::new(
                                    RichText::new("← Indietro")
                                        .size(16.0)
                                        .color(Color32::from_gray(220)),
                                )
                                    .min_size(vec2(btn_width, btn_height))
                                    .rounding(Rounding::same(10.0))
                                    .fill(Color32::from_rgb(50, 50, 50))
                                    .stroke(Stroke::new(1.0, Color32::from_gray(80)));

                                if ui.add(back_btn).clicked() {
                                    app.nav(View::Home);
                                }

                                ui.add_space(btn_spacing);

                                // Registrati
                                let reg_btn = egui::Button::new(
                                    RichText::new("📝  Registrati")
                                        .size(16.0)
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                    .min_size(vec2(btn_width, btn_height))
                                    .rounding(Rounding::same(10.0))
                                    .fill(Color32::from_rgb(0, 110, 210));

                                if ui.add_enabled(!app.is_loading, reg_btn).clicked() {
                                    let tx = app.get_tx();
                                    app.is_loading = true;
                                    app.status = "Registrazione in corso…".into();

                                    let addr = app.server_addr.clone();
                                    let user = app.reg_username.clone();
                                    let pass = app.reg_password.clone();
                                    let handle = app.rt.as_ref().unwrap().handle().clone();
                                    let ctx2 = ctx.clone();

                                    handle.spawn(async move {
                                        let result = net::register(&addr, &user, &pass).await;
                                        let _ = tx.send(result);
                                        ctx2.request_repaint();
                                    });
                                }
                            });

                            // --- STATO / SPINNER ---
                            if app.is_loading {
                                ui.add_space(16.0);
                                ui.horizontal_centered(|ui| {
                                    ui.spinner();
                                    ui.label(&app.status);
                                });
                            } else if !app.status.is_empty() {
                                ui.add_space(16.0);
                                ui.label(&app.status);
                            }
                        });
                    });
                });
            });
    });
}
