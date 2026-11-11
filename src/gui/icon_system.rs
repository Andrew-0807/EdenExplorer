//! Axis's icon boundary.
//!
//! UI code should use [`AppIcon`] and the helpers in this module rather than
//! depending on a particular icon set, font, or egui integration crate.
//! Icons are glyphs of the vendored Lucide icon font
//! (`src/assets/fonts/lucide/`), drawn as text so they take the theme color.
//! The item viewer's file and folder icons keep their SVGs
//! ([`paint_svg_icon`]).

use crate::core::utils::widgets::paint_focus_ring;
use egui::emath::GuiRounding;
use egui::load::Bytes;
use egui::{
    Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Image, Rect, Response,
    Sense, Ui, Vec2, WidgetInfo, WidgetType,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum AppIcon {
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowCounterClockwise,
    ArrowsClockwise,
    ArrowsInLineHorizontal,
    CaretDown,
    CaretLeft,
    CaretRight,
    CaretUp,
    Check,
    Copy,
    ActionCut,
    ActionCopy,
    ActionPaste,
    ActionDelete,
    ActionRename,
    ActionProperties,
    DotsSixVertical,
    File,
    FilePlus,
    FileText,
    FolderOpen,
    FolderPlus,
    FolderSimple,
    /// Filled two-tone folder used by the Flat icon style.
    FolderFlat,
    Image,
    ImageSquare,
    ImagesSquare,
    Info,
    List,
    Minus,
    Moon,
    MusicNotes,
    Palette,
    PencilSimple,
    Plus,
    PushPin,
    Question,
    SidebarSimple,
    Sliders,
    SplitHorizontal,
    Square,
    Star,
    Sun,
    Tabs,
    Tag,
    Terminal,
    Trash,
    VideoCamera,
    X,
    DownloadSimple,
}

impl AppIcon {
    #[cfg(test)]
    pub const ALL: &'static [Self] = &[
        Self::ArrowLeft,
        Self::ArrowRight,
        Self::ArrowUp,
        Self::ArrowCounterClockwise,
        Self::ArrowsClockwise,
        Self::ArrowsInLineHorizontal,
        Self::CaretDown,
        Self::CaretLeft,
        Self::CaretRight,
        Self::CaretUp,
        Self::Check,
        Self::Copy,
        Self::ActionCut,
        Self::ActionCopy,
        Self::ActionPaste,
        Self::ActionDelete,
        Self::ActionRename,
        Self::ActionProperties,
        Self::DotsSixVertical,
        Self::File,
        Self::FilePlus,
        Self::FileText,
        Self::FolderOpen,
        Self::FolderPlus,
        Self::FolderSimple,
        Self::FolderFlat,
        Self::Image,
        Self::ImageSquare,
        Self::ImagesSquare,
        Self::Info,
        Self::List,
        Self::Minus,
        Self::Moon,
        Self::MusicNotes,
        Self::Palette,
        Self::PencilSimple,
        Self::Plus,
        Self::PushPin,
        Self::Question,
        Self::SidebarSimple,
        Self::Sliders,
        Self::SplitHorizontal,
        Self::Square,
        Self::Star,
        Self::Sun,
        Self::Tabs,
        Self::Tag,
        Self::Terminal,
        Self::Trash,
        Self::VideoCamera,
        Self::X,
        Self::DownloadSimple,
    ];
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum IconStyle {
    #[default]
    Regular,
    Filled,
}

impl AppIcon {
    pub fn regular(self) -> StyledIcon {
        StyledIcon {
            icon: self,
            style: IconStyle::Regular,
        }
    }

    pub fn filled(self) -> StyledIcon {
        StyledIcon {
            icon: self,
            style: IconStyle::Filled,
        }
    }

    /// The Lucide font codepoint, from `font/info.json` of `lucide-static`
    /// 1.62.0. Codepoints change between Lucide releases, so update them
    /// together with `src/assets/fonts/lucide/lucide.ttf`. `None` for icons
    /// Lucide has no glyph for; those are drawn from their SVG.
    fn glyph(self) -> Option<char> {
        let code = match self {
            Self::ArrowLeft => '\u{e048}',                     // arrow-left
            Self::ArrowRight => '\u{e049}',                    // arrow-right
            Self::ArrowUp => '\u{e04a}',                       // arrow-up
            Self::ArrowCounterClockwise => '\u{e2a1}',         // undo-2
            Self::ArrowsClockwise => '\u{e145}',               // refresh-cw
            Self::ArrowsInLineHorizontal => '\u{e21c}',        // panel-left-close
            Self::CaretDown => '\u{e06d}',                     // chevron-down
            Self::CaretLeft => '\u{e06e}',                     // chevron-left
            Self::CaretRight => '\u{e06f}',                    // chevron-right
            Self::CaretUp => '\u{e070}',                       // chevron-up
            Self::Check => '\u{e06c}',                         // check
            Self::Copy | Self::ActionCopy => '\u{e09e}',       // copy
            Self::ActionCut => '\u{e14e}',                     // scissors
            Self::ActionPaste => '\u{e3e8}',                   // clipboard-paste
            Self::ActionDelete => '\u{e18e}',                  // trash-2
            Self::ActionRename => '\u{e4f0}',                  // pencil-line
            Self::ActionProperties | Self::Info => '\u{e0f9}', // info
            Self::DotsSixVertical => '\u{e0eb}',               // grip-vertical
            Self::File => '\u{e0c0}',                          // file
            Self::FilePlus => '\u{e0c9}',                      // file-plus
            Self::FileText => '\u{e0cc}',                      // file-text
            Self::FolderOpen => '\u{e247}',                    // folder-open
            Self::FolderPlus => '\u{e0d9}',                    // folder-plus
            Self::FolderSimple => '\u{e0d7}',                  // folder
            Self::FolderFlat => return None,
            Self::Image | Self::ImageSquare => '\u{e0f6}', // image
            Self::ImagesSquare => '\u{e5c4}',              // images
            Self::List => '\u{e106}',                      // list
            Self::Minus => '\u{e11c}',                     // minus
            Self::Moon => '\u{e11e}',                      // moon
            Self::MusicNotes => '\u{e55a}',                // audio-lines
            Self::Palette => '\u{e1dd}',                   // palette
            Self::PencilSimple => '\u{e1f9}',              // pencil
            Self::Plus => '\u{e13d}',                      // plus
            Self::PushPin => '\u{e259}',                   // pin
            Self::Question => '\u{e082}',                  // circle-question-mark
            Self::SidebarSimple => '\u{e12a}',             // panel-left
            Self::Sliders => '\u{e29a}',                   // sliders-horizontal
            Self::SplitHorizontal => '\u{e098}',           // columns-2
            Self::Square => '\u{e167}',                    // square
            Self::Star => '\u{e176}',                      // star
            Self::Sun => '\u{e178}',                       // sun
            Self::Tabs => '\u{e435}',                      // panel-top
            Self::Tag => '\u{e17f}',                       // tag
            Self::Terminal => '\u{e20a}',                  // square-terminal
            Self::Trash => '\u{e18d}',                     // trash
            Self::VideoCamera => '\u{e1a5}',               // video
            Self::X => '\u{e1b2}',                         // x
            Self::DownloadSimple => '\u{e0b2}',            // download
        };
        Some(code)
    }

    /// SVG sources kept for the item viewer's file and folder icons
    /// (`folder_icons.rs` and the gallery fallbacks), which keep their SVG
    /// look. `FolderFlat` is a filled shape with no Lucide glyph.
    fn svg(self) -> Option<(&'static str, &'static str)> {
        let pair = match self {
            Self::FolderFlat => (
                "folder-flat",
                include_str!("../assets/icons/lucide/folder-flat.svg"),
            ),
            Self::FolderSimple => ("folder", include_str!("../assets/icons/lucide/folder.svg")),
            Self::FileText => (
                "file-text",
                include_str!("../assets/icons/lucide/file-text.svg"),
            ),
            Self::DownloadSimple => (
                "download",
                include_str!("../assets/icons/lucide/download.svg"),
            ),
            Self::Image | Self::ImageSquare => {
                ("image", include_str!("../assets/icons/lucide/image.svg"))
            }
            Self::MusicNotes => (
                "audio-lines",
                include_str!("../assets/icons/lucide/audio-lines.svg"),
            ),
            Self::VideoCamera => ("video", include_str!("../assets/icons/lucide/video.svg")),
            Self::Square => ("square", include_str!("../assets/icons/lucide/square.svg")),
            _ => return None,
        };
        Some(pair)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StyledIcon {
    pub icon: AppIcon,
    pub style: IconStyle,
}

/// Name of the egui font family that holds the Lucide icon font.
const ICON_FONT: &str = "lucide";

static ICON_FAMILY: LazyLock<FontFamily> = LazyLock::new(|| FontFamily::Name(ICON_FONT.into()));

/// Registers the Lucide icon font as its own family. Call wherever the app
/// builds its `FontDefinitions`, after any fallback fonts are appended to
/// every family, so the icon family holds only Lucide.
pub fn install_icon_font(fonts: &mut FontDefinitions) {
    fonts.font_data.insert(
        ICON_FONT.to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/lucide/lucide.ttf"
        ))),
    );
    fonts
        .families
        .insert(ICON_FAMILY.clone(), vec![ICON_FONT.to_owned()]);
}

fn icon_rect(ui: &mut Ui, size: f32) -> (Rect, Response) {
    ui.allocate_exact_size(Vec2::splat(size), Sense::click())
}

pub fn draw_icon(
    ui: &mut Ui,
    icon: StyledIcon,
    size: f32,
    color: Color32,
    label: &str,
) -> Response {
    let (rect, response) = icon_rect(ui, size);
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), label));
    paint_icon(ui, rect, icon, size, color);
    paint_focus_ring(ui, rect, &response, CornerRadius::ZERO);
    response
}

/// Paints `icon` as a Lucide font glyph, `size` square, centered in `rect`.
/// The glyph takes `color` and scales with `size`. The style is ignored:
/// Lucide is stroke-only.
pub fn paint_icon(ui: &mut Ui, rect: Rect, icon: StyledIcon, size: f32, color: Color32) {
    match icon.icon.glyph() {
        Some(glyph) => paint_glyph(ui, rect, glyph, size, color),
        None => paint_svg_icon(ui, rect, icon, size, color),
    }
}

fn paint_glyph(ui: &Ui, rect: Rect, glyph: char, size: f32, color: Color32) {
    let painter = ui.painter();
    // A context whose fonts were not built by the app (unit tests) has no
    // icon family; egui would panic on it.
    if !painter.fonts(|f| f.definitions().families.contains_key(&*ICON_FAMILY)) {
        return;
    }
    let ppp = ui.ctx().pixels_per_point();
    // Lucide's em is its 24-unit grid (ascent = em, descent = 0), so the
    // galley is exactly `size` square. A whole number of device pixels keeps
    // the 2-unit strokes on the same grid at every size.
    let px = (size * ppp).round().max(1.0) / ppp;
    let galley = painter.layout_no_wrap(
        glyph.to_string(),
        FontId::new(px, ICON_FAMILY.clone()),
        color,
    );
    let pos = (rect.center() - galley.size() / 2.0).round_to_pixels(ppp);
    painter.galley(pos, galley, color);
}

/// URI and SVG payload for an icon tinted with one color.
type ColoredIconSource = (String, Arc<[u8]>);

thread_local! {
    /// Colored SVG sources per icon and RGBA color, built once per pair.
    static COLORED_ICON_SOURCES: RefCell<HashMap<(AppIcon, [u8; 4]), ColoredIconSource>> =
        RefCell::new(HashMap::new());
}

/// Builds the `bytes://` URI and SVG payload for `icon` tinted with `color`.
fn build_colored_icon_source(icon: AppIcon, color: Color32) -> Option<ColoredIconSource> {
    let (name, svg) = icon.svg()?;
    let color_key = format!(
        "{:02x}{:02x}{:02x}{:02x}",
        color.r(),
        color.g(),
        color.b(),
        color.a()
    );
    // Lucide SVGs use `currentColor`. An SVG loaded as an image has no
    // inherited CSS color, so replace it before handing the bytes to the
    // loader. Tinting black SVG pixels would remain black.
    let color_svg = svg.replace("currentColor", &format!("#{color_key}"));
    let uri = format!("bytes://eden-icon/{name}-{color_key}.svg");
    Some((uri, Arc::from(color_svg.into_bytes())))
}

fn colored_icon_source(icon: AppIcon, color: Color32) -> Option<ColoredIconSource> {
    COLORED_ICON_SOURCES.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(source) = cache.get(&(icon, color.to_array())) {
            return Some(source.clone());
        }
        let source = build_colored_icon_source(icon, color)?;
        cache.insert((icon, color.to_array()), source.clone());
        Some(source)
    })
}

/// Paints `icon` from its SVG. Only for the item viewer's file and folder
/// icons, which keep their SVG look; icons without an SVG use the glyph.
pub fn paint_svg_icon(ui: &mut Ui, rect: Rect, icon: StyledIcon, size: f32, color: Color32) {
    let Some((uri, bytes)) = colored_icon_source(icon.icon, color) else {
        if let Some(glyph) = icon.icon.glyph() {
            paint_glyph(ui, rect, glyph, size, color);
        }
        return;
    };
    let image = Image::from_bytes(uri, Bytes::from(bytes)).fit_to_exact_size(Vec2::splat(size));
    // Callers may already have allocated their layout rect (sidebar rows), so
    // paint into an icon-sized rect at its center instead of allocating.
    let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(size));
    image.paint_at(ui, icon_rect);
}

#[cfg(test)]
mod tests {
    use super::{AppIcon, ICON_FAMILY, install_icon_font};

    #[test]
    fn colored_icon_source_matches_direct_build_and_is_cached() {
        use super::{build_colored_icon_source, colored_icon_source};
        use egui::Color32;
        use std::sync::Arc;

        let colors = [
            Color32::from_rgba_unmultiplied(0x12, 0x34, 0x56, 0xff),
            Color32::from_rgba_unmultiplied(0xab, 0xcd, 0xef, 0x80),
        ];
        for icon in [AppIcon::FolderFlat, AppIcon::FolderSimple, AppIcon::Image] {
            for color in colors {
                let (uri, bytes) = colored_icon_source(icon, color).unwrap();
                let (direct_uri, direct_bytes) = build_colored_icon_source(icon, color).unwrap();
                assert_eq!(uri, direct_uri);
                assert_eq!(&bytes[..], &direct_bytes[..]);

                let text = String::from_utf8(bytes.to_vec()).unwrap();
                assert!(!text.contains("currentColor"));
                assert!(text.contains(&format!(
                    "#{:02x}{:02x}{:02x}{:02x}",
                    color.r(),
                    color.g(),
                    color.b(),
                    color.a()
                )));
                assert!(uri.starts_with("bytes://eden-icon/"));

                let (_, again) = colored_icon_source(icon, color).unwrap();
                assert!(
                    Arc::ptr_eq(&bytes, &again),
                    "second lookup must reuse the cached SVG"
                );
            }
        }
        assert!(colored_icon_source(AppIcon::Star, colors[0]).is_none());
    }

    /// Every icon is either a glyph present in the bundled font or an SVG.
    #[test]
    fn every_app_icon_has_a_glyph_or_svg() {
        let ctx = egui::Context::default();
        let mut fonts = egui::FontDefinitions::default();
        install_icon_font(&mut fonts);
        ctx.set_fonts(fonts);
        let _ = ctx.run_ui(Default::default(), |_| {});
        let font_id = egui::FontId::new(16.0, ICON_FAMILY.clone());
        for icon in AppIcon::ALL {
            match icon.glyph() {
                Some(glyph) => assert!(
                    ctx.fonts_mut(|f| f.has_glyph(&font_id, glyph)),
                    "Lucide font has no glyph for {icon:?}"
                ),
                None => {
                    let (_, svg) = icon.svg().expect("icon without glyph needs an SVG");
                    assert!(svg.trim_start().starts_with("<svg"), "bad SVG for {icon:?}");
                }
            }
        }
    }
}
