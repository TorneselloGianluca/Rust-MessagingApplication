use eframe::egui::{self, Align, Color32, Layout, RichText, Rounding, Stroke, vec2, FontFamily};
use crate::app::{App, View};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    // 1. Occupa tutto lo spazio e centra il contenuto (la Card) verticalmente e orizzontalmente
    ui.centered_and_justified(|ui| {

        // 2. La "Card" (il riquadro grigio scuro)
        egui::Frame::none()
            .fill(ui.style().visuals.window_fill().linear_multiply(1.3)) // Colore sfondo leggermente staccato dal fondo
            .rounding(Rounding::same(20.0)) // Angoli arrotondati
            .stroke(Stroke::new(1.0, Color32::from_white_alpha(15))) // Bordo sottile elegante
            .inner_margin(40.0) // Margine interno abbondante
            .show(ui, |ui| {

                // 3. Layout Verticale Centrato per il contenuto della card
                ui.vertical_centered(|ui| {

                    // --- LOGO / EMOJI ---
                    ui.label(
                        RichText::new("🦀")
                            .family(FontFamily::Proportional)
                            .size(80.0)
                    );

                    ui.add_space(15.0);

                    // --- TITOLO ---
                    ui.heading(
                        RichText::new("Ruggine")
                            .size(42.0)
                            .strong()
                            .color(Color32::from_gray(240))
                    );

                    ui.add_space(8.0);

                    // --- SOTTOTITOLO ---
                    ui.label(
                        RichText::new("Chat sicura, veloce e... ossidata.")
                            .size(15.0)
                            .color(Color32::from_gray(170))
                            .italics()
                    );

                    ui.add_space(40.0); // Spazio tra testo e bottoni

                    // --- BOTTONI CENTRATI SOTTO AL TESTO ---
                    // Dimensioni e spaziatura dei pulsanti
                    let btn_width = 130.0;
                    let btn_height = 45.0;
                    let btn_spacing = 20.0;
                    let total_width = 2.0 * btn_width + btn_spacing;

                    ui.horizontal(|ui| {
                        // Calcola padding a sinistra per centrare il gruppo di pulsanti
                        let available = ui.available_width();
                        let left_pad = ((available - total_width).max(0.0)) / 2.0;
                        ui.add_space(left_pad);

                        // Bottone LOGIN
                        let login_btn = egui::Button::new(
                            RichText::new("🔐  Login")
                                .size(16.0)
                                .strong()
                                .color(Color32::WHITE)
                        )
                            .min_size(vec2(btn_width, btn_height))
                            .rounding(Rounding::same(12.0))
                            .fill(Color32::from_rgb(0, 110, 210)); // Blu acceso

                        if ui.add(login_btn).on_hover_text("Accedi").clicked() {
                            app.nav(View::Login);
                        }

                        ui.add_space(btn_spacing);

                        // Bottone REGISTRATI
                        let reg_btn = egui::Button::new(
                            RichText::new("📝  Registrati")
                                .size(16.0)
                                .strong()
                                .color(Color32::from_gray(220))
                        )
                            .min_size(vec2(btn_width, btn_height))
                            .rounding(Rounding::same(12.0))
                            .fill(Color32::from_rgb(50, 50, 50)) // Grigio scuro
                            .stroke(Stroke::new(1.0, Color32::from_gray(80)));

                        if ui.add(reg_btn).on_hover_text("Crea account").clicked() {
                            app.nav(View::Register);
                        }
                    });
                });
            });
    });
}
