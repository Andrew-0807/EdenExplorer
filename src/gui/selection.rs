//! How selected and hovered items are drawn (fill, outline or both).

use crate::gui::theme::ThemePalette;
use eframe::egui::{Color32, CornerRadius, Stroke};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionStyle {
    /// Filled background (EdenExplorer's classic look).
    #[default]
    Fill,
    /// Outline only, like Explorer with translucent styling.
    Outline,
    FillAndOutline,
}

impl SelectionStyle {
    pub const ALL: [SelectionStyle; 3] = [
        SelectionStyle::Fill,
        SelectionStyle::Outline,
        SelectionStyle::FillAndOutline,
    ];

    pub fn label_key(self) -> &'static str {
        match self {
            SelectionStyle::Fill => "theme_selection_fill",
            SelectionStyle::Outline => "theme_selection_outline",
            SelectionStyle::FillAndOutline => "theme_selection_fill_outline",
        }
    }

    fn fills(self) -> bool {
        matches!(self, SelectionStyle::Fill | SelectionStyle::FillAndOutline)
    }

    fn outlines(self) -> bool {
        matches!(
            self,
            SelectionStyle::Outline | SelectionStyle::FillAndOutline
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum HoverStyle {
    #[default]
    Fill,
    Outline,
    None,
}

impl HoverStyle {
    pub const ALL: [HoverStyle; 3] = [HoverStyle::Fill, HoverStyle::Outline, HoverStyle::None];

    pub fn label_key(self) -> &'static str {
        match self {
            HoverStyle::Fill => "theme_hover_fill",
            HoverStyle::Outline => "theme_hover_outline",
            HoverStyle::None => "theme_hover_none",
        }
    }
}

/// What to paint behind one row or tile.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ItemPaint {
    pub fill: Option<Color32>,
    pub stroke: Option<Stroke>,
    pub radius: CornerRadius,
}

impl ItemPaint {
    pub const NONE: ItemPaint = ItemPaint {
        fill: None,
        stroke: None,
        radius: CornerRadius::ZERO,
    };

    pub fn is_empty(&self) -> bool {
        self.fill.is_none() && self.stroke.is_none()
    }
}

/// Decides how an item is highlighted. `hover` is the 0..1 hover amount
/// (`motion::hover`), which fades the hover color in and out. `selection_fill`
/// and `hover_fill` are the colors of the current egui visuals (derived from
/// the palette).
pub fn item_paint(
    palette: &ThemePalette,
    selected: bool,
    hover: f32,
    selection_fill: Color32,
    hover_fill: Color32,
) -> ItemPaint {
    let radius = CornerRadius::same(palette.selection_radius);
    let stroke = Stroke::new(
        palette.selection_stroke_width.max(0.5),
        palette.selection_stroke,
    );
    if selected {
        let style = palette.selection_style;
        return ItemPaint {
            fill: style.fills().then_some(selection_fill),
            stroke: style.outlines().then_some(stroke),
            radius,
        };
    }
    if hover > 0.0 {
        let hover_fill = hover_fill.gamma_multiply(hover.min(1.0));
        return match palette.hover_style {
            HoverStyle::Fill => ItemPaint {
                fill: Some(hover_fill),
                stroke: None,
                radius,
            },
            HoverStyle::Outline => ItemPaint {
                fill: None,
                stroke: Some(Stroke::new(stroke.width, hover_fill)),
                radius,
            },
            HoverStyle::None => ItemPaint::NONE,
        };
    }
    ItemPaint::NONE
}

/// Paints `paint` into `rect` (stroke drawn inside the rect so neighbors do
/// not overlap).
pub fn paint_item(painter: &eframe::egui::Painter, rect: eframe::egui::Rect, paint: ItemPaint) {
    if let Some(fill) = paint.fill {
        painter.rect_filled(rect, paint.radius, fill);
    }
    if let Some(stroke) = paint.stroke {
        painter.rect_stroke(rect, paint.radius, stroke, eframe::egui::StrokeKind::Inside);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::theme::{ThemeMode, get_default_palette};

    const SEL: Color32 = Color32::from_rgb(1, 1, 1);
    const HOV: Color32 = Color32::from_rgb(2, 2, 2);

    #[test]
    fn default_is_the_classic_fill() {
        let p = get_default_palette(ThemeMode::Dark);
        let paint = item_paint(&p, true, 0.0, SEL, HOV);
        assert_eq!(paint.fill, Some(SEL));
        assert!(paint.stroke.is_none());
        let paint = item_paint(&p, false, 1.0, SEL, HOV);
        assert_eq!(paint.fill, Some(HOV));
        assert!(item_paint(&p, false, 0.0, SEL, HOV).is_empty());
    }

    #[test]
    fn outline_selection_has_no_fill() {
        let mut p = get_default_palette(ThemeMode::Dark);
        p.selection_style = SelectionStyle::Outline;
        p.selection_stroke_width = 1.5;
        let paint = item_paint(&p, true, 1.0, SEL, HOV);
        assert!(paint.fill.is_none());
        let stroke = paint.stroke.unwrap();
        assert_eq!(stroke.width, 1.5);
        assert_eq!(stroke.color, p.selection_stroke);
    }

    #[test]
    fn fill_and_outline_has_both_and_the_radius() {
        let mut p = get_default_palette(ThemeMode::Light);
        p.selection_style = SelectionStyle::FillAndOutline;
        p.selection_radius = 4;
        let paint = item_paint(&p, true, 0.0, SEL, HOV);
        assert_eq!(paint.fill, Some(SEL));
        assert!(paint.stroke.is_some());
        assert_eq!(paint.radius, CornerRadius::same(4));
    }

    #[test]
    fn hover_styles() {
        let mut p = get_default_palette(ThemeMode::Dark);
        p.hover_style = HoverStyle::Outline;
        let paint = item_paint(&p, false, 1.0, SEL, HOV);
        assert!(paint.fill.is_none());
        assert_eq!(paint.stroke.unwrap().color, HOV);
        p.hover_style = HoverStyle::None;
        assert!(item_paint(&p, false, 1.0, SEL, HOV).is_empty());
        // Selection wins over hover.
        assert_eq!(item_paint(&p, true, 1.0, SEL, HOV).fill, Some(SEL));
    }

    #[test]
    fn zero_width_stroke_is_still_visible() {
        let mut p = get_default_palette(ThemeMode::Dark);
        p.selection_style = SelectionStyle::Outline;
        p.selection_stroke_width = 0.0;
        assert_eq!(
            item_paint(&p, true, 0.0, SEL, HOV).stroke.unwrap().width,
            0.5
        );
    }
}
