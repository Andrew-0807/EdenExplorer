//! Rubber-band (marquee) selection for the details list: state and the pure
//! geometry behind it. All y values are content coordinates (0 is the top of
//! the first row), so rows that scroll out of view while dragging still count.

use std::collections::HashSet;
use std::path::PathBuf;

/// Distance from the top or bottom edge of the list where dragging scrolls it.
pub const AUTOSCROLL_EDGE: f32 = 32.0;
/// Scroll speed at the very edge, in points per second.
pub const AUTOSCROLL_MAX_SPEED: f32 = 900.0;
/// Small gap after the name text that still counts as the text, in points.
pub const NAME_END_PAD: f32 = 6.0;

#[derive(Clone, Default)]
pub struct Marquee {
    pub active: bool,
    /// Screen x where the drag started.
    pub origin_x: f32,
    /// Content y where the drag started.
    pub origin_y: f32,
    /// Ctrl held at the start: keep the previous selection.
    pub additive: bool,
    pub base: HashSet<PathBuf>,
    /// Scroll offset to apply next frame (auto-scroll).
    pub scroll_to: Option<f32>,
}

/// Inclusive range of rows whose box `[i * stride, i * stride + row_h]` overlaps
/// the content span `[y0, y1]`, or `None` when no row does.
pub fn rows_in_span(
    y0: f32,
    y1: f32,
    stride: f32,
    row_h: f32,
    count: usize,
) -> Option<(usize, usize)> {
    if count == 0 || stride <= 0.0 {
        return None;
    }
    let (lo, hi) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
    let last_bottom = (count - 1) as f32 * stride + row_h;
    if hi < 0.0 || lo > last_bottom {
        return None;
    }
    let first = ((lo - row_h) / stride).ceil().max(0.0) as usize;
    let last = ((hi / stride).floor().max(0.0) as usize).min(count - 1);
    // `first` can land on a row whose box ends just above `lo` due to rounding.
    let first = if (first as f32) * stride + row_h < lo {
        first + 1
    } else {
        first
    };
    (first <= last).then_some((first, last))
}

/// New selection: the hits, plus the previous selection when additive.
pub fn merged_selection(
    base: &HashSet<PathBuf>,
    hits: impl IntoIterator<Item = PathBuf>,
    additive: bool,
) -> HashSet<PathBuf> {
    let mut out = if additive {
        base.clone()
    } else {
        HashSet::new()
    };
    out.extend(hits);
    out
}

/// Scroll distance for this frame: negative scrolls up, positive down, zero when
/// the pointer is away from both edges. Speed ramps up toward and past the edge.
pub fn autoscroll_step(pointer_y: f32, top: f32, bottom: f32, dt: f32) -> f32 {
    let up = ((top + AUTOSCROLL_EDGE - pointer_y) / AUTOSCROLL_EDGE).clamp(0.0, 1.0);
    let down = ((pointer_y - (bottom - AUTOSCROLL_EDGE)) / AUTOSCROLL_EDGE).clamp(0.0, 1.0);
    (down - up) * AUTOSCROLL_MAX_SPEED * dt
}

/// A press at `x` in the Name cell `[cell_left, cell_right]` is on blank space
/// (so it starts a marquee instead of a file drag) when it is past the end of the
/// icon and text, which end at `content_end`.
pub fn is_blank_name_press(x: f32, content_end: f32, cell_right: f32) -> bool {
    x > content_end + NAME_END_PAD && x < cell_right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_overlapping_span() {
        // stride 28, row box 24 tall: row i covers [28i, 28i + 24]
        assert_eq!(rows_in_span(0.0, 10.0, 28.0, 24.0, 10), Some((0, 0)));
        assert_eq!(rows_in_span(10.0, 60.0, 28.0, 24.0, 10), Some((0, 2)));
        // y 25..27 is the gap between row 0 and row 1
        assert_eq!(rows_in_span(25.0, 27.0, 28.0, 24.0, 10), None);
        assert_eq!(rows_in_span(60.0, 10.0, 28.0, 24.0, 10), Some((0, 2)));
    }

    #[test]
    fn rows_span_clamps_to_list() {
        assert_eq!(rows_in_span(-50.0, 5.0, 28.0, 24.0, 3), Some((0, 0)));
        assert_eq!(rows_in_span(50.0, 5000.0, 28.0, 24.0, 3), Some((1, 2)));
        assert_eq!(rows_in_span(-50.0, -5.0, 28.0, 24.0, 3), None);
        assert_eq!(rows_in_span(500.0, 600.0, 28.0, 24.0, 3), None);
        assert_eq!(rows_in_span(0.0, 10.0, 28.0, 24.0, 0), None);
    }

    #[test]
    fn selection_replaces_or_adds() {
        let base: HashSet<PathBuf> = [PathBuf::from("a")].into();
        let hits = vec![PathBuf::from("b")];
        let replaced = merged_selection(&base, hits.clone(), false);
        assert_eq!(replaced, [PathBuf::from("b")].into());
        let added = merged_selection(&base, hits, true);
        assert_eq!(added, [PathBuf::from("a"), PathBuf::from("b")].into());
    }

    #[test]
    fn autoscroll_only_near_edges() {
        assert_eq!(autoscroll_step(200.0, 100.0, 400.0, 0.016), 0.0);
        assert!(autoscroll_step(105.0, 100.0, 400.0, 0.016) < 0.0);
        assert!(autoscroll_step(395.0, 100.0, 400.0, 0.016) > 0.0);
        // past the edge caps at full speed
        let capped = autoscroll_step(0.0, 100.0, 400.0, 1.0);
        assert_eq!(capped, -AUTOSCROLL_MAX_SPEED);
    }

    #[test]
    fn blank_press_is_after_text_inside_cell() {
        assert!(!is_blank_name_press(150.0, 160.0, 300.0));
        assert!(!is_blank_name_press(162.0, 160.0, 300.0));
        assert!(is_blank_name_press(200.0, 160.0, 300.0));
        assert!(!is_blank_name_press(310.0, 160.0, 300.0));
    }
}
