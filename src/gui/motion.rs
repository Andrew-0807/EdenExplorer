//! Shared motion timings, so hover and list animations move as one system
//! instead of each widget picking its own. Frames are paced by vsync.

use eframe::egui::{self, Id, emath::easing};

/// Hover highlight fade.
pub const HOVER_SECS: f32 = 0.08;
/// One row fading in after a folder opens.
pub const ITEM_APPEAR_SECS: f32 = 0.12;
/// Delay between consecutive rows of that fade.
pub const ITEM_STAGGER_SECS: f32 = 0.012;
/// Rows past this one share its delay, so long folders don't wait.
pub const ITEM_STAGGER_CAP: usize = 10;

/// 0..1 hover amount for `id`, eased out.
pub fn hover(ctx: &egui::Context, id: Id, hovered: bool) -> f32 {
    ctx.animate_bool_with_time_and_easing(id, hovered, HOVER_SECS, easing::cubic_out)
}

/// Opacity of visible row `row` (0 = top), `elapsed` seconds after the
/// listing appeared.
pub fn item_appear(elapsed: f32, row: usize) -> f32 {
    let delay = row.min(ITEM_STAGGER_CAP) as f32 * ITEM_STAGGER_SECS;
    easing::cubic_out(((elapsed - delay) / ITEM_APPEAR_SECS).clamp(0.0, 1.0))
}

const ITEM_APPEAR_TOTAL: f32 = ITEM_STAGGER_CAP as f32 * ITEM_STAGGER_SECS + ITEM_APPEAR_SECS;

/// Seconds since the listing `key` (e.g. a folder's hash) first showed
/// items, tracked in egui memory under `id`. Asks for frames until the
/// appear animation is done.
pub fn listing_age(ctx: &egui::Context, id: Id, key: u64, has_items: bool) -> f32 {
    let now = ctx.input(|i| i.time);
    let start = ctx.data_mut(|d| {
        let entry = d.get_temp_mut_or_insert_with(id, || (key, has_items, now));
        // A new folder, or the first items of one still loading.
        if entry.0 != key || (!entry.1 && has_items) {
            *entry = (key, has_items, now);
        }
        entry.1 = has_items;
        entry.2
    });
    let age = (now - start) as f32;
    if has_items && age < ITEM_APPEAR_TOTAL {
        ctx.request_repaint();
    }
    age
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_cascade_then_settle() {
        assert_eq!(item_appear(0.0, 0), 0.0);
        assert!(item_appear(ITEM_APPEAR_SECS * 0.5, 0) > item_appear(ITEM_APPEAR_SECS * 0.5, 3));
        assert_eq!(item_appear(ITEM_APPEAR_TOTAL, 500), 1.0);
        // Rows past the cap share its delay.
        assert_eq!(item_appear(0.15, ITEM_STAGGER_CAP), item_appear(0.15, 999));
    }
}
