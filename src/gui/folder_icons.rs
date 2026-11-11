//! Which icon a folder gets for the theme's icon style.

use crate::gui::icon_packs::{self, PackSvg};
use crate::gui::icon_system::{AppIcon, paint_svg_icon};
use crate::gui::theme::ThemePalette;
use eframe::egui::{self, Color32};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum IconStyle {
    /// Lucide glyphs for known folders, shell icons for everything else.
    #[default]
    Classic,
    /// Shell icons everywhere: exactly what Explorer shows, including icon
    /// packs (Windhawk icon-resource-redirect, desktop.ini IconResource).
    Native,
    /// Flat two-tone folders colored per known folder; shell icons for files.
    Flat,
}

impl IconStyle {
    pub const ALL: [IconStyle; 3] = [IconStyle::Classic, IconStyle::Native, IconStyle::Flat];

    pub fn label_key(self) -> &'static str {
        match self {
            IconStyle::Classic => "theme_icons_classic",
            IconStyle::Native => "theme_icons_native",
            IconStyle::Flat => "theme_icons_flat",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnownFolder {
    Desktop,
    Documents,
    Downloads,
    Pictures,
    Music,
    Videos,
}

impl KnownFolder {
    /// Matches the folder name (case-insensitive), as Explorer's defaults do.
    pub fn from_name(name: &str) -> Option<Self> {
        const NAMES: [(&str, KnownFolder); 6] = [
            ("Desktop", KnownFolder::Desktop),
            ("Documents", KnownFolder::Documents),
            ("Downloads", KnownFolder::Downloads),
            ("Pictures", KnownFolder::Pictures),
            ("Music", KnownFolder::Music),
            ("Videos", KnownFolder::Videos),
        ];
        NAMES
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, k)| *k)
    }

    pub fn glyph(self) -> AppIcon {
        match self {
            KnownFolder::Desktop => AppIcon::Square,
            KnownFolder::Documents => AppIcon::FileText,
            KnownFolder::Downloads => AppIcon::DownloadSimple,
            KnownFolder::Pictures => AppIcon::Image,
            KnownFolder::Music => AppIcon::MusicNotes,
            KnownFolder::Videos => AppIcon::VideoCamera,
        }
    }
}

/// Colors of the Flat style, one per known folder.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct KnownFolderColors {
    pub desktop: Color32,
    pub documents: Color32,
    pub downloads: Color32,
    pub pictures: Color32,
    pub music: Color32,
    pub videos: Color32,
}

impl Default for KnownFolderColors {
    /// Close to the Windows 11 known-folder icons.
    fn default() -> Self {
        Self {
            desktop: Color32::from_rgb(64, 156, 255),
            documents: Color32::from_rgb(120, 144, 176),
            downloads: Color32::from_rgb(46, 180, 110),
            pictures: Color32::from_rgb(40, 180, 190),
            music: Color32::from_rgb(240, 130, 60),
            videos: Color32::from_rgb(160, 100, 220),
        }
    }
}

impl KnownFolderColors {
    pub fn get(&self, folder: KnownFolder) -> Color32 {
        match folder {
            KnownFolder::Desktop => self.desktop,
            KnownFolder::Documents => self.documents,
            KnownFolder::Downloads => self.downloads,
            KnownFolder::Pictures => self.pictures,
            KnownFolder::Music => self.music,
            KnownFolder::Videos => self.videos,
        }
    }
}

/// What to draw for one item.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FolderIcon {
    /// Use the Windows shell icon.
    Shell,
    /// A single glyph in `color` (Classic known folders).
    Glyph(AppIcon, Color32),
    /// A flat folder in `color`, optionally with a small glyph on it.
    Flat {
        color: Color32,
        overlay: Option<AppIcon>,
    },
    /// An icon from the theme's icon pack (files and folders).
    Pack(&'static PackSvg),
}

/// Resolves the icon for an item named `name`. With an icon pack, files and
/// folders use it. Otherwise only directories get custom icons. Drive roots
/// (no name) always use the shell icon.
pub fn resolve_folder_icon(palette: &ThemePalette, name: Option<&str>, is_dir: bool) -> FolderIcon {
    if let Some(name) = name
        && let Some(svg) =
            icon_packs::find(&palette.icon_pack).and_then(|p| p.icon_for(name, is_dir))
    {
        return FolderIcon::Pack(svg);
    }
    if !is_dir {
        return FolderIcon::Shell;
    }
    let known = name.and_then(KnownFolder::from_name);
    match palette.icon_style {
        IconStyle::Native => FolderIcon::Shell,
        // Classic never had a Desktop glyph; keep it exactly as before.
        IconStyle::Classic => match known {
            Some(folder) if folder != KnownFolder::Desktop => {
                FolderIcon::Glyph(folder.glyph(), palette.icon_colored_hover)
            }
            _ => FolderIcon::Shell,
        },
        IconStyle::Flat => FolderIcon::Flat {
            color: known.map_or(palette.folder_color, |k| palette.known_folder_colors.get(k)),
            overlay: known.map(KnownFolder::glyph),
        },
    }
}

/// Convenience over [`resolve_folder_icon`] for a path. Drive roots and
/// paths without a file name always use the shell icon.
pub fn resolve_for_path(palette: &ThemePalette, path: &Path, is_dir: bool) -> FolderIcon {
    match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => resolve_folder_icon(palette, Some(name), is_dir),
        None => FolderIcon::Shell,
    }
}

/// Paints a non-shell icon into `rect` (`size` square, centered). Returns
/// false for [`FolderIcon::Shell`] so the caller draws the shell icon.
/// `dim` fades the icon (cut items).
pub fn paint_folder_icon(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    size: f32,
    icon: FolderIcon,
    dim: bool,
) -> bool {
    let fade = |c: Color32| if dim { c.linear_multiply(0.5) } else { c };
    match icon {
        FolderIcon::Shell => false,
        FolderIcon::Pack(svg) => icon_packs::paint(ui, rect, size, svg, fade(Color32::WHITE)),
        FolderIcon::Glyph(glyph, color) => {
            paint_svg_icon(ui, rect, glyph.regular(), size, fade(color));
            true
        }
        FolderIcon::Flat { color, overlay } => {
            paint_svg_icon(ui, rect, AppIcon::FolderFlat.regular(), size, fade(color));
            if let Some(glyph) = overlay {
                // Small glyph on the folder's front panel.
                let glyph_size = (size * 0.42).max(6.0);
                let center = rect.center() + egui::vec2(0.0, size * 0.1);
                let glyph_rect =
                    egui::Rect::from_center_size(center, egui::Vec2::splat(glyph_size));
                paint_svg_icon(
                    ui,
                    glyph_rect,
                    glyph.regular(),
                    glyph_size,
                    fade(Color32::from_white_alpha(235)),
                );
            }
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::theme::{ThemeMode, get_default_palette};

    fn palette(style: IconStyle) -> ThemePalette {
        let mut p = get_default_palette(ThemeMode::Dark);
        p.icon_style = style;
        p
    }

    #[test]
    fn known_folders_match_case_insensitively() {
        assert_eq!(KnownFolder::from_name("music"), Some(KnownFolder::Music));
        assert_eq!(
            KnownFolder::from_name("DOWNLOADS"),
            Some(KnownFolder::Downloads)
        );
        assert_eq!(KnownFolder::from_name("Musical"), None);
    }

    #[test]
    fn classic_keeps_the_previous_behavior() {
        let p = palette(IconStyle::Classic);
        assert_eq!(
            resolve_folder_icon(&p, Some("Music"), true),
            FolderIcon::Glyph(AppIcon::MusicNotes, p.icon_colored_hover)
        );
        assert_eq!(
            resolve_folder_icon(&p, Some("src"), true),
            FolderIcon::Shell
        );
        assert_eq!(
            resolve_folder_icon(&p, Some("Desktop"), true),
            FolderIcon::Shell
        );
    }

    #[test]
    fn native_always_uses_shell_icons() {
        let p = palette(IconStyle::Native);
        assert_eq!(
            resolve_folder_icon(&p, Some("Music"), true),
            FolderIcon::Shell
        );
    }

    #[test]
    fn flat_colors_known_and_plain_folders() {
        let mut p = palette(IconStyle::Flat);
        p.known_folder_colors.music = Color32::RED;
        assert_eq!(
            resolve_folder_icon(&p, Some("Music"), true),
            FolderIcon::Flat {
                color: Color32::RED,
                overlay: Some(AppIcon::MusicNotes)
            }
        );
        assert_eq!(
            resolve_folder_icon(&p, Some("projects"), true),
            FolderIcon::Flat {
                color: p.folder_color,
                overlay: None
            }
        );
    }

    #[test]
    fn files_and_drive_roots_use_shell_icons() {
        for style in IconStyle::ALL {
            let p = palette(style);
            assert_eq!(
                resolve_folder_icon(&p, Some("Music"), false),
                FolderIcon::Shell
            );
            assert_eq!(
                resolve_for_path(&p, Path::new("C:\\"), true),
                FolderIcon::Shell
            );
        }
    }
}
