mod app;
mod net;
mod views;

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use std::sync::Arc; // Importante per FontData

/// Funzione helper per configurare il tema all'avvio
fn setup_custom_theme(ctx: &egui::Context) {
    // --- 1. CARICAMENTO FONT EMOJI ---
    let mut fonts = FontDefinitions::default();


    if let Ok(font_data) = std::fs::read("./client/src/emoji.ttf") {
        println!("Caricamento emoji.ttf riuscito.");

        // Registra il font nel sistema di egui
        fonts.font_data.insert(
            "emoji_font".to_owned(),
            FontData::from_owned(font_data).tweak(
                // Tweak opzionale: scala leggermente le emoji se sembrano piccole
                egui::FontTweak {
                    scale: 1.0,
                    ..Default::default()
                }
            ),
        );

        // Aggiungi il font emoji in CODA alle famiglie di font.
        // Egui userà il primo font per il testo normale, e cercherà in "emoji_font"
        // solo se non trova il carattere nel primo (es. per il 🦀).

        // Per il testo proporzionale (bottoni, label standard)
        fonts.families
            .entry(FontFamily::Proportional)
            .or_default()
            .push("emoji_font".to_owned());

        // Per il testo monospazio (codice, log)
        fonts.families
            .entry(FontFamily::Monospace)
            .or_default()
            .push("emoji_font".to_owned());

    } else {
        println!("⚠️ 'emoji.ttf' non trovato! Le emoji potrebbero non apparire.");
    }

    // Applica i font
    ctx.set_fonts(fonts);

    // --- 2. IMPOSTAZIONE STILE E COLORI (Invariato) ---
    let mut style = (*ctx.style()).clone();
    let visuals = &mut style.visuals;

    visuals.dark_mode = true;
    visuals.override_text_color = Some(egui::Color32::from_gray(240));
    visuals.window_fill = egui::Color32::from_rgb(27, 27, 27);
    visuals.panel_fill = egui::Color32::from_rgb(27, 27, 27);
    visuals.extreme_bg_color = egui::Color32::from_rgb(18, 18, 18);

    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(27, 27, 27);
    visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::from_gray(240);
    visuals.widgets.noninteractive.rounding = egui::Rounding::same(8.0);

    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(42, 42, 42);
    visuals.widgets.inactive.rounding = egui::Rounding::same(8.0);

    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(60, 60, 60);
    visuals.widgets.hovered.rounding = egui::Rounding::same(8.0);

    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(80, 80, 80);
    visuals.widgets.active.rounding = egui::Rounding::same(8.0);

    visuals.selection.bg_fill = egui::Color32::from_rgb(0, 120, 215);
    visuals.selection.stroke.color = egui::Color32::WHITE;

    visuals.window_rounding = egui::Rounding::same(8.0);

    visuals.window_shadow = egui::epaint::Shadow {
        offset: egui::vec2(2.0, 2.0),
        blur: 5.0,
        spread: 1.0,
        color: egui::Color32::from_black_alpha(60),
    };

    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.window_margin = egui::Margin::same(12.0);

    ctx.set_style(style);
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("Ruggine"),
        ..Default::default()
    };

    eframe::run_native(
        "Ruggine",
        options,
        Box::new(|cc| {
            setup_custom_theme(&cc.egui_ctx);
            Box::<app::App>::default()
        }),
    )
}