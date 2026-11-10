use crate::core::drives::DriveInfo;
use crate::core::indexer::{DirectorySettingsSnapshot, WindowSizeMode};
use crate::gui::theme::{ThemeMode, ThemePalette};
use crate::gui::utils::{SortColumn, SortKey};
use crate::gui::windows::containers::enums::ItemViewerHeaderColumn;
use crate::gui::windows::containers::structs::FavoriteItem;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct AboutWindow {
    pub open: bool,
}

pub struct ThemeCustomizer {
    pub open: bool,
    pub selected_mode: ThemeMode,
    pub light_palette: ThemePalette,
    pub dark_palette: ThemePalette,
    /// Result of the last import/export: (i18n key, is_error).
    pub status: Option<(&'static str, bool)>,
}

impl Default for ThemeCustomizer {
    fn default() -> Self {
        Self {
            open: false,
            selected_mode: ThemeMode::Dark,
            light_palette: crate::gui::theme::get_palette(ThemeMode::Light),
            dark_palette: crate::gui::theme::get_palette(ThemeMode::Dark),
            status: None,
        }
    }
}

impl ThemeCustomizer {
    pub fn palette_mut(&mut self, mode: ThemeMode) -> &mut ThemePalette {
        match mode {
            ThemeMode::Dark => &mut self.dark_palette,
            ThemeMode::Light => &mut self.light_palette,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Navigation {
    pub current: PathBuf,
    pub back: Vec<PathBuf>,
    pub forward: Vec<PathBuf>,
}

/// Stored in `settings.json`. Missing fields take `AppSettings::default()`
/// values, so new fields can be added without a migration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub folder_scanning_enabled: bool,
    pub show_hidden_files_folders: bool,
    pub show_item_viewer_icons: bool,
    pub windows_context_menu_enabled: bool,
    #[serde(default)]
    pub extra_actions_bar: bool,
    pub start_path: Option<PathBuf>,
    pub window_size_mode: WindowSizeMode,
    pub pinned_tabs: Vec<PathBuf>,
    pub remember_open_tabs: bool,
    /// Records folders in zoxide after the user stays in them for a moment.
    pub zoxide_auto_add: bool,
    pub unremembered_tabs: Vec<PathBuf>,
    #[serde(default)]
    pub remembered_active_tab: Option<PathBuf>,
    pub time_format_24h: bool,
    pub date_style: crate::core::fs::DateStyle,
    pub sort_column: SortColumn,
    pub sort_ascending: bool,
    /// Full sort order applied to every folder; empty means `sort_column` alone.
    #[serde(default)]
    pub sort_keys: Vec<SortKey>,
    pub language: String,
    pub item_viewer_file_column_order: Vec<ItemViewerHeaderColumn>,
    pub item_viewer_drive_column_order: Vec<ItemViewerHeaderColumn>,
    pub recycle_bin_column_order: Vec<ItemViewerHeaderColumn>,
    pub item_viewer_file_column_sizes: Vec<f32>,
    pub item_viewer_drive_column_sizes: Vec<f32>,
    pub recycle_bin_column_sizes: Vec<f32>,
    pub directory_settings: Vec<DirectorySettingsSnapshot>,
    /// Toolbar, status bar and sidebar layout.
    pub ui: crate::gui::customization::UiPreferences,
}

#[derive(Default)]
pub struct SettingsWindow {
    pub open: bool,
    pub current_settings: AppSettings,
    pub show_reset_favorites_confirmation: bool,
    pub show_reset_confirmation: bool,
    pub show_clear_directory_cache_confirmation: bool,
    pub selected_tab: SettingsTab,
    /// Text in the settings search box; empty means normal tab navigation.
    pub search: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsTab {
    #[default]
    General,
    Appearance,
    ViewDefaults,
    Hotkeys,
}

pub struct SidebarState {
    pub favorites: Vec<FavoriteItem>,
    pub item_clicked: Option<PathBuf>,
    pub dragging_favorite: Option<usize>,
    /// Pointer offset from the dragged row's top, its current top y, and the
    /// per-favorite slide offsets (in rows) used by the reorder animation.
    pub drag_grab_dy: f32,
    pub drag_top: f32,
    pub row_offsets: std::collections::HashMap<PathBuf, f32>,
    pub sidebar_default_width: f32,
    pub cached_drives: Vec<DriveInfo>,
    pub last_drive_refresh: Instant,
    /// Result of a drive enumeration running on a worker thread, if one is pending.
    pub drive_refresh_rx: Option<std::sync::mpsc::Receiver<Vec<DriveInfo>>>,
    pub non_ntfs_popup_path: Option<PathBuf>,
    /// WSL distributions, read once on first draw.
    pub wsl_distros: Option<Vec<String>>,
    /// Favorite whose rename field is open in its context menu, and the text.
    pub rename_target: Option<PathBuf>,
    pub rename_buffer: String,
}

impl Default for SidebarState {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            favorites: vec![],
            dragging_favorite: None,
            drag_grab_dy: 0.0,
            drag_top: 0.0,
            row_offsets: std::collections::HashMap::new(),
            item_clicked: None,
            sidebar_default_width: 250.0,
            cached_drives: Vec::new(),
            last_drive_refresh: now.checked_sub(Duration::from_secs(60)).unwrap_or(now),
            drive_refresh_rx: None,
            non_ntfs_popup_path: None,
            wsl_distros: None,
            rename_target: None,
            rename_buffer: String::new(),
        }
    }
}
