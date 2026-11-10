use crate::gui::theme::ThemePalette;
use eframe::egui;
use std::time::{Duration, Instant};

const TOAST_DURATION: Duration = Duration::from_secs(3);
const TOAST_MARGIN: f32 = 12.0;

/// A short message at the bottom of the window that hides itself after ~3 s.
#[derive(Default)]
pub struct Toast {
    current: Option<(String, Instant)>,
}

impl Toast {
    /// Replaces the visible message and restarts the timer.
    pub fn show(&mut self, text: String) {
        self.current = Some((text, Instant::now()));
    }

    /// Draws the message if one is visible. Call once per frame.
    pub fn draw(&mut self, ctx: &egui::Context, palette: &ThemePalette) {
        let Some((text, shown)) = &self.current else {
            return;
        };
        let elapsed = shown.elapsed();
        if elapsed >= TOAST_DURATION {
            self.current = None;
            return;
        }
        // The window repaints only on input, so wake up to hide the message.
        ctx.request_repaint_after(TOAST_DURATION - elapsed);

        egui::Area::new(egui::Id::new("toast"))
            .order(egui::Order::Tooltip)
            .interactable(false)
            .anchor(egui::Align2::LEFT_BOTTOM, [TOAST_MARGIN, -TOAST_MARGIN])
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(palette.application_bg_color)
                    .stroke(egui::Stroke::new(1.0, palette.borders_default))
                    .corner_radius(egui::CornerRadius::same(palette.large_radius))
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(text.as_str())
                                .size(palette.text_size)
                                .color(palette.text_normal),
                        );
                    });
            });
    }
}
