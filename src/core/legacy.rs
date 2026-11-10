//! Frozen copies of the on-disk formats written by EdenExplorer v1.3.0 and
//! earlier, used only to read and migrate old files.
//!
//! DO NOT EDIT these structs. postcard is positional, so the field list, order
//! and types must stay byte-for-byte what v1.3.0 wrote. They are checked
//! against golden files in `core/testdata/` that were produced by the v1.3.0
//! structs. New fields belong in the live types, which are now stored as JSON
//! (see `core::persist`).

use crate::core::fs::DateStyle;
use crate::core::indexer::{DirectorySettingsSnapshot, WindowSizeMode};
use crate::gui::theme::{ThemeMode, ThemePalette};
use crate::gui::utils::{SortColumn, SortKey};
use crate::gui::windows::containers::enums::ItemViewerHeaderColumn;
use crate::gui::windows::containers::structs::{GalleryThumbnailSize, ItemViewerDisplayMode};
use crate::gui::windows::structs::AppSettings;
use eframe::egui::{Color32, CornerRadius};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// `THEME_VERSION` written into `theme.bin` by v1.3.0.
pub const THEME_BIN_VERSION_V8: u32 = 8;

/// Reads a legacy postcard file the way v1.3.0 did: bincode 1 first (files
/// from before the postcard switch), then postcard. Never writes anything.
fn decode_bincode_or_postcard<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Option<T> {
    if let Ok(value) = bincode::deserialize::<T>(bytes) {
        return Some(value);
    }
    postcard::from_bytes::<T>(bytes).ok()
}

// ---------------------------------------------------------------------------
// theme.bin (THEME_VERSION 8)
// ---------------------------------------------------------------------------

/// `ThemePalette` exactly as v1.3.0 declared it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThemePaletteV8 {
    pub text_size: f32,
    pub tooltip_text_size: f32,
    pub sidebar_icon_size: f32,
    pub tab_icon_size: f32,
    pub font_name: String,
    pub primary: Color32,
    pub primary_hover: Color32,
    pub primary_active: Color32,
    pub text_normal: Color32,
    pub text_header_section: Color32,
    pub application_bg_color: Color32,
    pub modal_background_effect_color: Color32,
    pub icon_color: Color32,
    pub icon_windows: Color32,
    pub icon_colored_hover: Color32,
    pub item_viewer_row_text_selected: Color32,
    pub tooltip_text_color: Color32,
    pub tab_text_selected: Color32,
    pub row_selected_bg: Color32,
    pub row_bg: Color32,
    pub row_height: f32,
    pub sidebar_item_spacing_y: f32,
    pub resize_handle: Color32,
    pub borders_default: Color32,
    pub borders_active: Color32,
    pub tab_close_hover: Color32,
    pub tab_close_active: Color32,
    pub tab_close_normal: Color32,
    pub tab_add_hover: Color32,
    pub pinned_tab_color: Color32,
    pub drive_usage_critical: Color32,
    pub drive_usage_warning: Color32,
    pub drive_usage_normal: Color32,
    pub drive_usage_background: Color32,
    pub drive_usage_text: Color32,
    pub checkbox_bg_default: Color32,
    pub checkbox_checkmark_color: Color32,
    pub checkbox_bg_hover: Color32,
    pub checkbox_bg_active: Color32,
    pub button_background: Color32,
    pub button_stroke: Color32,
    pub button_favorite_fill: Color32,
    pub button_seperator_handle_fill: Color32,
    pub small_radius: u8,
    pub medium_radius: u8,
    pub large_radius: u8,
    pub tab_active_radius: CornerRadius,
    pub tab_inactive_radius: CornerRadius,
    pub tab_button_radius: CornerRadius,
}

/// `ThemeSettingsSnapshot` exactly as v1.3.0 declared it.
#[derive(Serialize, Deserialize)]
pub struct ThemeSettingsSnapshotV8 {
    pub version: u32,
    pub light: ThemePaletteV8,
    pub dark: ThemePaletteV8,
}

impl ThemePaletteV8 {
    /// Upgrades to the current palette. Field names are unchanged, so the
    /// values are carried over through JSON; fields added since v1.3.0 take
    /// the defaults of `mode`.
    pub fn upgrade(&self, mode: ThemeMode) -> ThemePalette {
        let stored = serde_json::to_value(self).unwrap_or(serde_json::Value::Null);
        crate::gui::theme::palette_from_json(mode, stored)
    }
}

/// Decodes a v1.3.0 `theme.bin`. Returns `(light, dark)`, or `None` if the file
/// is unreadable or was written with another `THEME_VERSION` (whose layout is
/// unknown, exactly as v1.3.0 treated it).
pub fn decode_theme_bin(bytes: &[u8]) -> Option<(ThemePalette, ThemePalette)> {
    let snapshot = decode_bincode_or_postcard::<ThemeSettingsSnapshotV8>(bytes)?;
    if snapshot.version != THEME_BIN_VERSION_V8 {
        return None;
    }
    Some((
        snapshot.light.upgrade(ThemeMode::Light),
        snapshot.dark.upgrade(ThemeMode::Dark),
    ))
}

// ---------------------------------------------------------------------------
// settings.bin
// ---------------------------------------------------------------------------

/// `DirectorySettingsSnapshot` exactly as v1.3.0 declared it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DirectorySettingsSnapshotV130 {
    pub directory: PathBuf,
    pub item_viewer_file_column_order: Vec<ItemViewerHeaderColumn>,
    pub item_viewer_drive_column_order: Vec<ItemViewerHeaderColumn>,
    pub recycle_bin_column_order: Vec<ItemViewerHeaderColumn>,
    pub item_viewer_file_column_sizes: Vec<f32>,
    pub item_viewer_drive_column_sizes: Vec<f32>,
    pub recycle_bin_column_sizes: Vec<f32>,
    pub filter_query: String,
    pub display_mode: ItemViewerDisplayMode,
    pub gallery_thumbnail_size: GalleryThumbnailSize,
    pub sort_column: SortColumn,
    pub sort_ascending: bool,
    pub sort_keys: Vec<SortKey>,
}

impl From<DirectorySettingsSnapshotV130> for DirectorySettingsSnapshot {
    fn from(v: DirectorySettingsSnapshotV130) -> Self {
        Self {
            directory: v.directory,
            item_viewer_file_column_order: v.item_viewer_file_column_order,
            item_viewer_drive_column_order: v.item_viewer_drive_column_order,
            recycle_bin_column_order: v.recycle_bin_column_order,
            item_viewer_file_column_sizes: v.item_viewer_file_column_sizes,
            item_viewer_drive_column_sizes: v.item_viewer_drive_column_sizes,
            recycle_bin_column_sizes: v.recycle_bin_column_sizes,
            filter_query: v.filter_query,
            display_mode: v.display_mode,
            gallery_thumbnail_size: v.gallery_thumbnail_size,
            sort_column: v.sort_column,
            sort_ascending: v.sort_ascending,
            sort_keys: v.sort_keys,
        }
    }
}

/// `AppSettingsSnapshot` exactly as v1.3.0 declared it.
#[derive(Serialize, Deserialize)]
pub struct AppSettingsSnapshotV130 {
    pub folder_scanning_enabled: bool,
    pub show_hidden_files_folders: bool,
    pub show_item_viewer_icons: bool,
    pub windows_context_menu_enabled: bool,
    pub extra_actions_bar: bool,
    pub window_size_mode: WindowSizeMode,
    pub start_path: Option<PathBuf>,
    pub theme: Option<String>,
    pub pinned_tabs: Vec<PathBuf>,
    pub remember_open_tabs: bool,
    pub unremembered_tabs: Vec<PathBuf>,
    pub remembered_active_tab: Option<PathBuf>,
    pub time_format_24h: bool,
    pub date_style: DateStyle,
    pub sort_column: SortColumn,
    pub sort_ascending: bool,
    pub language: String,
    pub item_viewer_file_column_order: Vec<ItemViewerHeaderColumn>,
    pub item_viewer_drive_column_order: Vec<ItemViewerHeaderColumn>,
    pub recycle_bin_column_order: Vec<ItemViewerHeaderColumn>,
    pub item_viewer_file_column_sizes: Vec<f32>,
    pub item_viewer_drive_column_sizes: Vec<f32>,
    pub recycle_bin_column_sizes: Vec<f32>,
    pub directory_settings: Vec<DirectorySettingsSnapshotV130>,
}

/// The pre-1.0 settings layout (window size mode with `HalfScreen`), which
/// v1.3.0 still accepted as a fallback.
#[derive(Serialize, Deserialize)]
pub struct AppSettingsSnapshotV0 {
    pub folder_scanning_enabled: bool,
    pub windows_context_menu_enabled: bool,
    pub window_size_mode: WindowSizeModeV0,
    pub start_path: Option<PathBuf>,
    pub theme: Option<String>,
    pub pinned_tabs: Vec<PathBuf>,
    pub time_format_24h: bool,
    pub sort_column: SortColumn,
    pub sort_ascending: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WindowSizeModeV0 {
    FullScreen,
    HalfScreen,
    Custom { width: f32, height: f32 },
}

impl From<WindowSizeModeV0> for WindowSizeMode {
    fn from(legacy: WindowSizeModeV0) -> Self {
        match legacy {
            WindowSizeModeV0::FullScreen => WindowSizeMode::FullScreen,
            WindowSizeModeV0::HalfScreen => WindowSizeMode::Custom {
                width: 960.0,
                height: 540.0,
            },
            WindowSizeModeV0::Custom { width, height } => WindowSizeMode::Custom { width, height },
        }
    }
}

impl AppSettingsSnapshotV130 {
    /// Converts to the live settings model plus the saved theme name. Fields
    /// added to `AppSettings` after v1.3.0 take their defaults.
    #[allow(clippy::needless_update)]
    pub fn upgrade(self) -> (AppSettings, Option<String>) {
        let settings = AppSettings {
            folder_scanning_enabled: self.folder_scanning_enabled,
            show_hidden_files_folders: self.show_hidden_files_folders,
            show_item_viewer_icons: self.show_item_viewer_icons,
            windows_context_menu_enabled: self.windows_context_menu_enabled,
            extra_actions_bar: self.extra_actions_bar,
            start_path: self.start_path,
            window_size_mode: self.window_size_mode,
            pinned_tabs: self.pinned_tabs,
            remember_open_tabs: self.remember_open_tabs,
            unremembered_tabs: self.unremembered_tabs,
            remembered_active_tab: self.remembered_active_tab,
            time_format_24h: self.time_format_24h,
            date_style: self.date_style,
            sort_column: self.sort_column,
            sort_ascending: self.sort_ascending,
            language: self.language,
            item_viewer_file_column_order: self.item_viewer_file_column_order,
            item_viewer_drive_column_order: self.item_viewer_drive_column_order,
            recycle_bin_column_order: self.recycle_bin_column_order,
            item_viewer_file_column_sizes: self.item_viewer_file_column_sizes,
            item_viewer_drive_column_sizes: self.item_viewer_drive_column_sizes,
            recycle_bin_column_sizes: self.recycle_bin_column_sizes,
            directory_settings: self
                .directory_settings
                .into_iter()
                .map(Into::into)
                .collect(),
            ..AppSettings::default()
        };
        (settings, self.theme)
    }
}

impl AppSettingsSnapshotV0 {
    pub fn upgrade(self) -> (AppSettings, Option<String>) {
        let defaults = AppSettings::default();
        let settings = AppSettings {
            folder_scanning_enabled: self.folder_scanning_enabled,
            windows_context_menu_enabled: self.windows_context_menu_enabled,
            window_size_mode: self.window_size_mode.into(),
            start_path: self.start_path,
            pinned_tabs: self.pinned_tabs,
            time_format_24h: self.time_format_24h,
            sort_column: self.sort_column,
            sort_ascending: self.sort_ascending,
            // v1.3.0 used these values when it upgraded this layout.
            show_hidden_files_folders: true,
            show_item_viewer_icons: true,
            extra_actions_bar: false,
            ..defaults
        };
        (settings, self.theme)
    }
}

/// Decodes a `settings.bin` written by v1.3.0 or earlier, trying the layouts
/// in the order v1.3.0 did.
pub fn decode_settings_bin(bytes: &[u8]) -> Option<(AppSettings, Option<String>)> {
    if let Some(snapshot) = decode_bincode_or_postcard::<AppSettingsSnapshotV130>(bytes) {
        return Some(snapshot.upgrade());
    }
    decode_bincode_or_postcard::<AppSettingsSnapshotV0>(bytes).map(AppSettingsSnapshotV0::upgrade)
}

// ---------------------------------------------------------------------------
// favorites\drive_C.bin
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
pub struct FavoritesSnapshotV130 {
    pub favorites: Vec<String>,
}

/// Decodes `favorites\drive_C.bin` into the list of favorite paths.
pub fn decode_favorites_bin(bytes: &[u8]) -> Option<Vec<String>> {
    decode_bincode_or_postcard::<FavoritesSnapshotV130>(bytes).map(|s| s.favorites)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::theme::get_default_palette;

    /// Golden files written by the v1.3.0 structs (see the test below for the
    /// values they contain).
    const THEME_V130: &[u8] = include_bytes!("testdata/theme_v1_3_0.bin");
    const SETTINGS_V130: &[u8] = include_bytes!("testdata/settings_v1_3_0.bin");
    const FAVORITES_V130: &[u8] = include_bytes!("testdata/favorites_v1_3_0.bin");

    #[test]
    fn frozen_theme_struct_matches_the_golden_v1_3_0_bytes() {
        // Decoding and re-encoding with the frozen struct must reproduce the
        // exact bytes, otherwise the struct drifted from what v1.3.0 wrote.
        let snapshot: ThemeSettingsSnapshotV8 = postcard::from_bytes(THEME_V130).unwrap();
        assert_eq!(postcard::to_allocvec(&snapshot).unwrap(), THEME_V130);
    }

    #[test]
    fn frozen_settings_struct_matches_the_golden_v1_3_0_bytes() {
        let snapshot: AppSettingsSnapshotV130 = postcard::from_bytes(SETTINGS_V130).unwrap();
        assert_eq!(postcard::to_allocvec(&snapshot).unwrap(), SETTINGS_V130);
        let favorites: FavoritesSnapshotV130 = postcard::from_bytes(FAVORITES_V130).unwrap();
        assert_eq!(postcard::to_allocvec(&favorites).unwrap(), FAVORITES_V130);
    }

    #[test]
    fn v1_3_0_theme_upgrades_losslessly() {
        let (light, dark) = decode_theme_bin(THEME_V130).expect("decodes");

        // Values customised in the golden file survive.
        assert_eq!(dark.primary, Color32::from_rgb(1, 2, 3));
        assert_eq!(dark.text_size, 13.5);
        assert_eq!(dark.font_name, "Cascadia Code");
        assert_eq!(dark.row_height, 19.0);
        assert_eq!(dark.application_bg_color, Color32::from_rgb(10, 20, 30));
        assert_eq!(
            dark.tab_active_radius,
            CornerRadius {
                nw: 1,
                ne: 2,
                sw: 3,
                se: 4
            }
        );
        assert_eq!(light.primary, Color32::from_rgb(200, 100, 50));
        assert_eq!(light.medium_radius, 7);
        assert_eq!(light.sidebar_item_spacing_y, 1.25);

        // Every v1.3.0 field is carried over unchanged.
        let snapshot: ThemeSettingsSnapshotV8 = postcard::from_bytes(THEME_V130).unwrap();
        for (old, new) in [(&snapshot.dark, &dark), (&snapshot.light, &light)] {
            let old_json = serde_json::to_value(old).unwrap();
            let new_json = serde_json::to_value(new).unwrap();
            for (key, value) in old_json.as_object().unwrap() {
                assert_eq!(new_json.get(key), Some(value), "field {key} changed");
            }
        }
    }

    #[test]
    fn theme_with_another_version_is_not_misread() {
        let mut snapshot: ThemeSettingsSnapshotV8 = postcard::from_bytes(THEME_V130).unwrap();
        snapshot.version = 7;
        let bytes = postcard::to_allocvec(&snapshot).unwrap();
        assert!(decode_theme_bin(&bytes).is_none());
        assert!(decode_theme_bin(b"").is_none());
        assert!(decode_theme_bin(&[0xff; 7]).is_none());
    }

    #[test]
    fn v1_3_0_settings_upgrade_losslessly() {
        let (settings, theme) = decode_settings_bin(SETTINGS_V130).expect("decodes");
        assert_eq!(theme.as_deref(), Some("light"));
        assert!(!settings.folder_scanning_enabled);
        assert!(!settings.show_hidden_files_folders);
        assert!(settings.windows_context_menu_enabled);
        assert!(settings.extra_actions_bar);
        assert!(matches!(
            settings.window_size_mode,
            WindowSizeMode::Custom {
                width: 1234.0,
                height: 777.0
            }
        ));
        assert_eq!(settings.start_path, Some(PathBuf::from("D:\\Work\\データ")));
        assert_eq!(
            settings.pinned_tabs,
            vec![PathBuf::from("C:\\Users"), PathBuf::from("E:\\")]
        );
        assert!(settings.remember_open_tabs);
        assert_eq!(settings.unremembered_tabs, vec![PathBuf::from("C:\\Temp")]);
        assert_eq!(settings.remembered_active_tab, Some(PathBuf::from("E:\\")));
        assert!(settings.time_format_24h);
        assert_eq!(settings.date_style, DateStyle::Iso);
        assert_eq!(settings.sort_column, SortColumn::Size);
        assert!(!settings.sort_ascending);
        assert_eq!(settings.language, "ja-JP");
        assert_eq!(
            settings.item_viewer_file_column_order,
            vec![
                ItemViewerHeaderColumn::Modified,
                ItemViewerHeaderColumn::Type
            ]
        );
        assert_eq!(
            settings.item_viewer_file_column_sizes,
            vec![300.0, 61.0, 76.0, 101.0, 102.0, 103.0]
        );
        assert_eq!(settings.directory_settings.len(), 1);
        let dir = &settings.directory_settings[0];
        assert_eq!(dir.directory, PathBuf::from("C:\\Pictures"));
        assert_eq!(dir.filter_query, "jpg");
        assert_eq!(dir.display_mode, ItemViewerDisplayMode::Gallery);
        assert_eq!(dir.gallery_thumbnail_size, GalleryThumbnailSize::Large);
        assert_eq!(dir.sort_column, SortColumn::Modified);
        assert_eq!(dir.sort_keys.len(), 1);
    }

    #[test]
    fn pre_1_0_settings_still_upgrade() {
        let old = AppSettingsSnapshotV0 {
            folder_scanning_enabled: false,
            windows_context_menu_enabled: true,
            window_size_mode: WindowSizeModeV0::HalfScreen,
            start_path: None,
            theme: Some("dark".into()),
            pinned_tabs: vec![PathBuf::from("C:\\")],
            time_format_24h: true,
            sort_column: SortColumn::Name,
            sort_ascending: true,
        };
        let bytes = postcard::to_allocvec(&old).unwrap();
        let (settings, theme) = decode_settings_bin(&bytes).expect("decodes");
        assert_eq!(theme.as_deref(), Some("dark"));
        assert!(!settings.folder_scanning_enabled);
        assert!(matches!(
            settings.window_size_mode,
            WindowSizeMode::Custom {
                width: 960.0,
                height: 540.0
            }
        ));
        assert!(settings.show_hidden_files_folders);
    }

    #[test]
    fn favorites_decode() {
        assert_eq!(
            decode_favorites_bin(FAVORITES_V130).unwrap(),
            vec![
                "C:\\Users\\Andrew\\Desktop".to_string(),
                "G:\\projects".to_string()
            ]
        );
    }

    #[test]
    fn upgrade_of_default_palette_is_the_default() {
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            let current = get_default_palette(mode);
            let v8: ThemePaletteV8 =
                serde_json::from_value(serde_json::to_value(&current).unwrap()).unwrap();
            let upgraded = v8.upgrade(mode);
            assert_eq!(
                serde_json::to_value(&upgraded).unwrap(),
                serde_json::to_value(&current).unwrap()
            );
        }
    }
}
