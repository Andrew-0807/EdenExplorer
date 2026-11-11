//! Rubber-band (marquee) selection for the details list: state and the pure
//! geometry behind it. All y values are content coordinates (0 is the top of
//! the first row), so rows that scroll out of view while dragging still count.

use eframe::egui;
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

/// Layout of a tile grid, in content coordinates (0,0 is the top-left of the
/// first tile). Tile `i` sits at column `i % columns`, row `i / columns`.
#[derive(Clone, Copy)]
pub struct TileGrid {
    pub tile_w: f32,
    pub tile_h: f32,
    pub col_pitch: f32,
    pub row_pitch: f32,
    pub columns: usize,
    pub count: usize,
}

/// Indices of the tiles overlapped by the rectangle spanned by the two corner
/// points, in ascending order.
pub fn tiles_in_rect(a: (f32, f32), b: (f32, f32), grid: TileGrid) -> Vec<usize> {
    if grid.columns == 0 || grid.count == 0 {
        return Vec::new();
    }
    let rows = grid.count.div_ceil(grid.columns);
    let Some((c0, c1)) = rows_in_span(a.0, b.0, grid.col_pitch, grid.tile_w, grid.columns) else {
        return Vec::new();
    };
    let Some((r0, r1)) = rows_in_span(a.1, b.1, grid.row_pitch, grid.tile_h, rows) else {
        return Vec::new();
    };
    (r0..=r1)
        .flat_map(|r| (c0..=c1).map(move |c| r * grid.columns + c))
        .filter(|&i| i < grid.count)
        .collect()
}

/// Draws the rubber band rectangle.
pub fn paint_marquee(
    painter: &egui::Painter,
    rect: egui::Rect,
    palette: &crate::gui::theme::ThemePalette,
) {
    painter.rect_filled(rect, 0.0, palette.primary.linear_multiply(0.2));
    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, palette.primary),
        egui::StrokeKind::Inside,
    );
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

/// Where Ctrl+Up/Down moves the keyboard focus. `focus` is the current focus
/// row, `down` the direction, `selectable` tells which rows may take focus.
pub fn ctrl_focus_target(
    focus: Option<usize>,
    count: usize,
    down: bool,
    selectable: impl Fn(usize) -> bool,
) -> Option<usize> {
    if count == 0 {
        return None;
    }
    match focus.filter(|&f| f < count) {
        None => {
            if down {
                (0..count).find(|&i| selectable(i))
            } else {
                (0..count).rev().find(|&i| selectable(i))
            }
        }
        Some(f) if down => (f + 1..count).find(|&i| selectable(i)),
        Some(f) => (0..f).rev().find(|&i| selectable(i)),
    }
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

    fn grid() -> TileGrid {
        // 3 columns, 7 tiles; tiles 100x80 with a 10 gap
        TileGrid {
            tile_w: 100.0,
            tile_h: 80.0,
            col_pitch: 110.0,
            row_pitch: 90.0,
            columns: 3,
            count: 7,
        }
    }

    #[test]
    fn tile_rect_hits_block_of_tiles() {
        assert_eq!(tiles_in_rect((10.0, 10.0), (50.0, 50.0), grid()), vec![0]);
        assert_eq!(
            tiles_in_rect((50.0, 50.0), (150.0, 120.0), grid()),
            vec![0, 1, 3, 4]
        );
        // corners given in reverse order
        assert_eq!(
            tiles_in_rect((150.0, 120.0), (50.0, 50.0), grid()),
            vec![0, 1, 3, 4]
        );
    }

    #[test]
    fn tile_rect_ignores_gaps_and_missing_tiles() {
        // x 101..109 is the gap between columns 0 and 1
        assert!(tiles_in_rect((101.0, 10.0), (109.0, 50.0), grid()).is_empty());
        // the last row holds only tile 6; columns 1..2 of that row are empty
        assert!(tiles_in_rect((120.0, 185.0), (320.0, 200.0), grid()).is_empty());
        assert_eq!(tiles_in_rect((0.0, 185.0), (320.0, 200.0), grid()), vec![6]);
        assert!(tiles_in_rect((400.0, 0.0), (500.0, 50.0), grid()).is_empty());
        assert!(tiles_in_rect((-50.0, -50.0), (-5.0, -5.0), grid()).is_empty());
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

    #[test]
    fn ctrl_focus_moves_and_stops_at_ends() {
        let all = |_: usize| true;
        assert_eq!(ctrl_focus_target(Some(2), 5, true, all), Some(3));
        assert_eq!(ctrl_focus_target(Some(2), 5, false, all), Some(1));
        assert_eq!(ctrl_focus_target(Some(4), 5, true, all), None);
        assert_eq!(ctrl_focus_target(Some(0), 5, false, all), None);
        assert_eq!(ctrl_focus_target(None, 5, true, all), Some(0));
        assert_eq!(ctrl_focus_target(None, 5, false, all), Some(4));
        // skips rows that cannot take focus
        assert_eq!(ctrl_focus_target(Some(0), 5, true, |i| i != 1), Some(2));
    }
}
