//! On-disk state under `%LOCALAPPDATA%\ExplorerEden` (despite the name, there
//! is no search index here).
//!
//! | File | Format |
//! |---|---|
//! | `settings.json` | versioned JSON (`eden-settings`), migrated from `settings.bin` |
//! | `theme.json` | versioned JSON (`eden-theme-store`), migrated from `theme.bin` |
//! | `quick_access.json` | versioned JSON (`eden-quick-access`), migrated from `favorites\drive_C.bin` |
//! | `tags.bin`, `window_position.bin`, `window_state.bin` | postcard (unchanged layouts) |
//!
//! Legacy files are only read, never modified or deleted, so a downgrade keeps
//! working. Every write is atomic (`persist::write_atomic`).

use crate::core::fs::MY_PC_PATH;
use crate::core::legacy;
use crate::core::persist::{
    self, EnvelopeError, from_value_with_defaults, parse_envelope, preserve_unreadable,
    write_atomic,
};
use crate::gui::theme::{THEME_VERSION, ThemeMode, ThemePalette, palette_from_json};
use crate::gui::utils::SortKey;
use crate::gui::windows::containers::enums::ItemViewerHeaderColumn;
use crate::gui::windows::containers::structs::{GalleryThumbnailSize, ItemViewerDisplayMode};
use crate::gui::windows::structs::AppSettings;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const SETTINGS_FILE: &str = "settings.json";
const SETTINGS_LEGACY_FILE: &str = "settings.bin";
const SETTINGS_FORMAT: &str = "eden-settings";
/// Version 1 was the postcard `settings.bin`.
pub const SETTINGS_VERSION: u32 = 2;

const THEME_FILE: &str = "theme.json";
const THEME_LEGACY_FILE: &str = "theme.bin";
const THEME_STORE_FORMAT: &str = "eden-theme-store";

const QUICK_ACCESS_FILE: &str = "quick_access.json";
const QUICK_ACCESS_FORMAT: &str = "eden-quick-access";
const QUICK_ACCESS_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
pub struct TagGroupSnapshot {
    pub id: u64,
    pub name: String,
    pub color: [u8; 4],
    pub items: Vec<PathBuf>,
}

#[derive(Serialize, Deserialize)]
pub struct TagsSnapshot {
    #[serde(default = "default_tags_version")]
    pub version: u32,
    #[serde(default = "default_next_tag_group_id")]
    pub next_group_id: u64,
    #[serde(default)]
    pub groups: Vec<TagGroupSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DirectorySettingsSnapshot {
    pub directory: PathBuf,
    #[serde(default = "default_item_viewer_file_column_order")]
    pub item_viewer_file_column_order: Vec<ItemViewerHeaderColumn>,
    #[serde(default = "default_item_viewer_drive_column_order")]
    pub item_viewer_drive_column_order: Vec<ItemViewerHeaderColumn>,
    #[serde(default = "default_recycle_bin_column_order")]
    pub recycle_bin_column_order: Vec<ItemViewerHeaderColumn>,
    #[serde(default = "default_item_viewer_file_column_size")]
    pub item_viewer_file_column_sizes: Vec<f32>,
    #[serde(default = "default_item_viewer_drive_column_size")]
    pub item_viewer_drive_column_sizes: Vec<f32>,
    #[serde(default = "default_recycle_bin_column_size")]
    pub recycle_bin_column_sizes: Vec<f32>,
    #[serde(default)]
    pub filter_query: String,
    #[serde(default)]
    pub display_mode: ItemViewerDisplayMode,
    #[serde(default = "default_gallery_thumbnail_size")]
    pub gallery_thumbnail_size: GalleryThumbnailSize,
    #[serde(default = "default_sort_column")]
    pub sort_column: crate::gui::utils::SortColumn,
    #[serde(default)]
    pub sort_ascending: bool,
    #[serde(default)]
    pub sort_keys: Vec<SortKey>,
}

fn default_gallery_thumbnail_size() -> GalleryThumbnailSize {
    GalleryThumbnailSize::Medium
}

#[derive(Serialize, Deserialize)]
struct WindowPositionSnapshot {
    x: i32,
    y: i32,
}

#[derive(Serialize, Deserialize)]
struct WindowStateSnapshot {
    was_maximized: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WindowSizeMode {
    FullScreen,
    Custom { width: f32, height: f32 },
}

impl Default for WindowSizeMode {
    fn default() -> Self {
        Self::Custom {
            width: 1200.0,
            height: 800.0,
        }
    }
}

/// Reads one of the small postcard files (tags, window state). Files from the
/// bincode era are converted to postcard in place, atomically.
fn load_or_migrate_bincode_to_postcard<T>(path: &Path) -> Option<T>
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    let data = std::fs::read(path).ok()?;

    // 1️⃣ Try OLD format first (bincode)
    if let Ok(v) = bincode::deserialize::<T>(&data) {
        if let Ok(new_bytes) = postcard::to_allocvec(&v) {
            let _ = write_atomic(path, &new_bytes);
        }
        return Some(v);
    }

    // 2️⃣ NEW format (postcard)
    postcard::from_bytes::<T>(&data).ok()
}

fn save_postcard<T: Serialize>(path: &Path, value: &T) {
    if let Ok(data) = postcard::to_allocvec(value)
        && let Err(err) = write_atomic(path, &data)
    {
        eprintln!("Failed to save {}: {err}", path.display());
    }
}

fn default_sort_column() -> crate::gui::utils::SortColumn {
    crate::gui::utils::SortColumn::Name
}

fn default_item_viewer_file_column_order() -> Vec<ItemViewerHeaderColumn> {
    vec![
        ItemViewerHeaderColumn::Type,
        ItemViewerHeaderColumn::Size,
        ItemViewerHeaderColumn::Tags,
        ItemViewerHeaderColumn::Modified,
        ItemViewerHeaderColumn::Created,
    ]
}

fn default_item_viewer_drive_column_order() -> Vec<ItemViewerHeaderColumn> {
    vec![
        ItemViewerHeaderColumn::Type,
        ItemViewerHeaderColumn::Size,
        ItemViewerHeaderColumn::Usage,
    ]
}

fn default_recycle_bin_column_order() -> Vec<ItemViewerHeaderColumn> {
    vec![
        ItemViewerHeaderColumn::OriginalDirectory,
        ItemViewerHeaderColumn::Type,
        ItemViewerHeaderColumn::Size,
        ItemViewerHeaderColumn::Deleted,
        ItemViewerHeaderColumn::Created,
    ]
}

pub fn default_item_viewer_file_column_size() -> Vec<f32> {
    vec![180.0, 60.0, 75.0, 100.0, 100.0, 100.0]
}

pub fn default_item_viewer_drive_column_size() -> Vec<f32> {
    vec![180.0, 120.0, 150.0]
}

pub fn default_recycle_bin_column_size() -> Vec<f32> {
    vec![180.0, 200.0, 60.0, 120.0, 100.0, 100.0]
}

fn default_tags_version() -> u32 {
    1
}

fn default_next_tag_group_id() -> u64 {
    1
}

/// A file from a newer build still loads (unknown fields are ignored), but
/// note it for anyone debugging from a console.
fn warn_if_newer(file: &str, found: u32, supported: u32) {
    if found > supported {
        eprintln!("{file} has version {found}, newer than {supported}; unknown fields ignored");
    }
}

fn data_file(name: &str) -> Option<PathBuf> {
    persist::data_dir().map(|dir| dir.join(name))
}

// ---------------------------------------------------------------------------
// Window position / state (read in main.rs before the window exists)
// ---------------------------------------------------------------------------

pub fn load_window_position_on_start() -> Option<(i32, i32)> {
    let path = data_file("window_position.bin")?;
    load_or_migrate_bincode_to_postcard::<WindowPositionSnapshot>(&path)
        .map(|snapshot| (snapshot.x, snapshot.y))
}

pub fn save_window_position(x: i32, y: i32) {
    if let Some(path) = data_file("window_position.bin") {
        save_postcard(&path, &WindowPositionSnapshot { x, y });
    }
}

pub fn load_window_was_maximized_on_start() -> bool {
    let Some(path) = data_file("window_state.bin") else {
        return false;
    };
    load_or_migrate_bincode_to_postcard::<WindowStateSnapshot>(&path)
        .map(|snapshot| snapshot.was_maximized)
        .unwrap_or(false)
}

pub fn save_window_was_maximized(was_maximized: bool) {
    if let Some(path) = data_file("window_state.bin") {
        save_postcard(&path, &WindowStateSnapshot { was_maximized });
    }
}

// ---------------------------------------------------------------------------
// Tags
// ---------------------------------------------------------------------------

pub fn load_tags() -> Option<TagsSnapshot> {
    let path = data_file("tags.bin")?;
    load_or_migrate_bincode_to_postcard::<TagsSnapshot>(&path)
}

pub fn save_tags(snapshot: &TagsSnapshot) {
    if let Some(path) = data_file("tags.bin") {
        save_postcard(&path, snapshot);
    }
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

/// Everything stored in `settings.json`.
#[derive(Clone, Debug)]
pub struct StoredSettings {
    pub settings: AppSettings,
    /// Active theme mode name ("dark" / "light").
    pub theme: Option<String>,
}

impl Default for StoredSettings {
    fn default() -> Self {
        Self {
            settings: normalize_settings(AppSettings::default()),
            theme: None,
        }
    }
}

#[derive(Serialize)]
struct SettingsBody<'a> {
    theme: Option<&'a str>,
    settings: &'a AppSettings,
}

fn normalize_settings(mut settings: AppSettings) -> AppSettings {
    if settings.start_path.is_none() {
        settings.start_path = Some(PathBuf::from(MY_PC_PATH));
    }
    settings
}

/// Loads the settings, migrating a v1.3.0 `settings.bin` on first run.
pub fn load_settings() -> StoredSettings {
    match persist::data_dir() {
        Some(dir) => load_settings_from(&dir),
        None => StoredSettings::default(),
    }
}

pub fn save_settings(stored: &StoredSettings) {
    if let Some(dir) = persist::data_dir()
        && let Err(err) = save_settings_to(&dir, stored)
    {
        eprintln!("Failed to save settings: {err}");
    }
}

pub(crate) fn load_settings_from(dir: &Path) -> StoredSettings {
    let json_path = dir.join(SETTINGS_FILE);
    if let Ok(bytes) = std::fs::read(&json_path) {
        match parse_envelope(&bytes, SETTINGS_FORMAT) {
            Ok(envelope) => {
                warn_if_newer("settings.json", envelope.version, SETTINGS_VERSION);
                let theme = envelope
                    .body
                    .get("theme")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
                let stored = envelope
                    .body
                    .get("settings")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                let merged = from_value_with_defaults(&AppSettings::default(), stored);
                if !merged.rejected.is_empty() {
                    eprintln!(
                        "settings.json: ignored invalid fields {:?}",
                        merged.rejected
                    );
                }
                return StoredSettings {
                    settings: normalize_settings(merged.value),
                    theme,
                };
            }
            Err(EnvelopeError::Malformed | EnvelopeError::WrongFormat) => {
                preserve_unreadable(&json_path);
            }
        }
    }

    if let Some((settings, theme)) = std::fs::read(dir.join(SETTINGS_LEGACY_FILE))
        .ok()
        .and_then(|bytes| legacy::decode_settings_bin(&bytes))
    {
        let stored = StoredSettings {
            settings: normalize_settings(settings),
            theme,
        };
        // Write the JSON copy now; settings.bin is left as it is.
        let _ = save_settings_to(dir, &stored);
        return stored;
    }

    StoredSettings::default()
}

pub(crate) fn save_settings_to(dir: &Path, stored: &StoredSettings) -> std::io::Result<()> {
    let body = SettingsBody {
        theme: stored.theme.as_deref(),
        settings: &stored.settings,
    };
    let bytes = persist::envelope_bytes(SETTINGS_FORMAT, SETTINGS_VERSION, &body)
        .ok_or_else(|| std::io::Error::other("settings could not be serialized"))?;
    write_atomic(&dir.join(SETTINGS_FILE), &bytes)
}

pub fn load_windows_size_mode_on_start() -> WindowSizeMode {
    load_settings().settings.window_size_mode
}

// ---------------------------------------------------------------------------
// Quick access (sidebar favorites)
// ---------------------------------------------------------------------------

/// One pinned sidebar entry. `label` is only stored when the user renamed it;
/// otherwise the folder name is shown.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuickAccessEntry {
    pub path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct QuickAccessBody {
    #[serde(default)]
    items: Vec<QuickAccessEntry>,
}

/// The pinned list, or `None` if it was never saved (the caller then seeds
/// the default folders). Migrates v1.3.0's `favorites\drive_C.bin`.
pub fn load_quick_access() -> Option<Vec<QuickAccessEntry>> {
    load_quick_access_from(&persist::data_dir()?)
}

pub fn save_quick_access(items: &[QuickAccessEntry]) {
    if let Some(dir) = persist::data_dir()
        && let Err(err) = save_quick_access_to(&dir, items)
    {
        eprintln!("Failed to save quick access: {err}");
    }
}

pub(crate) fn load_quick_access_from(dir: &Path) -> Option<Vec<QuickAccessEntry>> {
    let json_path = dir.join(QUICK_ACCESS_FILE);
    if let Ok(bytes) = std::fs::read(&json_path) {
        match parse_envelope(&bytes, QUICK_ACCESS_FORMAT) {
            Ok(envelope) => {
                let body: QuickAccessBody =
                    serde_json::from_value(serde_json::Value::Object(envelope.body))
                        .unwrap_or(QuickAccessBody { items: Vec::new() });
                return Some(body.items);
            }
            Err(_) => preserve_unreadable(&json_path),
        }
    }

    // v1.3.0 always used drive 'C'.
    let legacy_path = dir.join("favorites").join("drive_C.bin");
    let paths = std::fs::read(legacy_path)
        .ok()
        .and_then(|bytes| legacy::decode_favorites_bin(&bytes))?;
    // An empty legacy list meant "seed the defaults", exactly as before.
    if paths.is_empty() {
        return None;
    }
    let items: Vec<QuickAccessEntry> = paths
        .into_iter()
        .map(|path| QuickAccessEntry {
            path: PathBuf::from(path),
            label: None,
        })
        .collect();
    let _ = save_quick_access_to(dir, &items);
    Some(items)
}

pub(crate) fn save_quick_access_to(dir: &Path, items: &[QuickAccessEntry]) -> std::io::Result<()> {
    let body = QuickAccessBody {
        items: items.to_vec(),
    };
    let bytes = persist::envelope_bytes(QUICK_ACCESS_FORMAT, QUICK_ACCESS_VERSION, &body)
        .ok_or_else(|| std::io::Error::other("quick access could not be serialized"))?;
    write_atomic(&dir.join(QUICK_ACCESS_FILE), &bytes)
}

// ---------------------------------------------------------------------------
// Theme
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct ThemeStoreBody<'a> {
    light: &'a ThemePalette,
    dark: &'a ThemePalette,
}

/// Loads `(light, dark)`, migrating a v1.3.0 `theme.bin` on first run. `None`
/// means nothing usable was found and the defaults apply.
pub fn load_theme_settings() -> Option<(ThemePalette, ThemePalette)> {
    load_theme_settings_from(&persist::data_dir()?)
}

pub fn save_theme_settings(light: &ThemePalette, dark: &ThemePalette) {
    if let Some(dir) = persist::data_dir()
        && let Err(err) = save_theme_settings_to(&dir, light, dark)
    {
        eprintln!("Failed to save theme: {err}");
    }
}

pub(crate) fn load_theme_settings_from(dir: &Path) -> Option<(ThemePalette, ThemePalette)> {
    let json_path = dir.join(THEME_FILE);
    if let Ok(bytes) = std::fs::read(&json_path) {
        match parse_envelope(&bytes, THEME_STORE_FORMAT) {
            Ok(mut envelope) => {
                warn_if_newer("theme.json", envelope.version, THEME_VERSION);
                let light = envelope
                    .body
                    .remove("light")
                    .unwrap_or(serde_json::Value::Null);
                let dark = envelope
                    .body
                    .remove("dark")
                    .unwrap_or(serde_json::Value::Null);
                return Some((
                    palette_from_json(ThemeMode::Light, light),
                    palette_from_json(ThemeMode::Dark, dark),
                ));
            }
            Err(_) => preserve_unreadable(&json_path),
        }
    }

    let (light, dark) = std::fs::read(dir.join(THEME_LEGACY_FILE))
        .ok()
        .and_then(|bytes| legacy::decode_theme_bin(&bytes))?;
    // Write the JSON copy now; theme.bin is left as it is.
    let _ = save_theme_settings_to(dir, &light, &dark);
    Some((light, dark))
}

pub(crate) fn save_theme_settings_to(
    dir: &Path,
    light: &ThemePalette,
    dark: &ThemePalette,
) -> std::io::Result<()> {
    let body = ThemeStoreBody { light, dark };
    let bytes = persist::envelope_bytes(THEME_STORE_FORMAT, THEME_VERSION, &body)
        .ok_or_else(|| std::io::Error::other("theme could not be serialized"))?;
    write_atomic(&dir.join(THEME_FILE), &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::persist::test_dir::TempDir;
    use crate::gui::theme::get_default_palette;
    use eframe::egui::Color32;

    const THEME_V130: &[u8] = include_bytes!("testdata/theme_v1_3_0.bin");
    const SETTINGS_V130: &[u8] = include_bytes!("testdata/settings_v1_3_0.bin");
    const FAVORITES_V130: &[u8] = include_bytes!("testdata/favorites_v1_3_0.bin");

    #[test]
    fn v1_3_0_theme_bin_migrates_and_is_left_untouched() {
        let dir = TempDir::new("theme-migrate");
        std::fs::write(dir.path().join("theme.bin"), THEME_V130).unwrap();

        let (light, dark) = load_theme_settings_from(dir.path()).expect("migrated");
        assert_eq!(dark.primary, Color32::from_rgb(1, 2, 3));
        assert_eq!(dark.font_name, "Cascadia Code");
        assert_eq!(light.medium_radius, 7);

        // The legacy file is byte-for-byte unchanged and a JSON copy exists.
        assert_eq!(
            std::fs::read(dir.path().join("theme.bin")).unwrap(),
            THEME_V130
        );
        assert!(dir.path().join("theme.json").exists());

        // The next start reads the JSON and gets the same palettes.
        let (light2, dark2) = load_theme_settings_from(dir.path()).unwrap();
        assert_eq!(
            serde_json::to_value(&dark).unwrap(),
            serde_json::to_value(&dark2).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&light).unwrap(),
            serde_json::to_value(&light2).unwrap()
        );
    }

    #[test]
    fn theme_json_round_trip() {
        let dir = TempDir::new("theme-roundtrip");
        let mut dark = get_default_palette(ThemeMode::Dark);
        dark.primary = Color32::from_rgb(9, 8, 7);
        let light = get_default_palette(ThemeMode::Light);
        save_theme_settings_to(dir.path(), &light, &dark).unwrap();
        let (_, loaded_dark) = load_theme_settings_from(dir.path()).unwrap();
        assert_eq!(loaded_dark.primary, Color32::from_rgb(9, 8, 7));
    }

    #[test]
    fn theme_json_with_missing_and_unknown_fields_keeps_the_rest() {
        let dir = TempDir::new("theme-partial");
        let json = r#"{
            "format": "eden-theme-store",
            "version": 99,
            "dark": { "primary": [1, 2, 3, 255], "text_size": "bad", "from_the_future": 1 }
        }"#;
        std::fs::write(dir.path().join("theme.json"), json).unwrap();
        let (light, dark) = load_theme_settings_from(dir.path()).unwrap();
        let defaults = get_default_palette(ThemeMode::Dark);
        assert_eq!(dark.primary, Color32::from_rgba_premultiplied(1, 2, 3, 255));
        assert_eq!(dark.text_size, defaults.text_size);
        assert_eq!(dark.row_height, defaults.row_height);
        assert_eq!(light.primary, get_default_palette(ThemeMode::Light).primary);
    }

    #[test]
    fn unreadable_theme_json_is_preserved_and_legacy_is_used() {
        let dir = TempDir::new("theme-corrupt");
        std::fs::write(dir.path().join("theme.json"), b"{ not json").unwrap();
        std::fs::write(dir.path().join("theme.bin"), THEME_V130).unwrap();
        let (_, dark) = load_theme_settings_from(dir.path()).unwrap();
        assert_eq!(dark.primary, Color32::from_rgb(1, 2, 3));
        assert_eq!(
            std::fs::read(dir.path().join("theme.json.unreadable")).unwrap(),
            b"{ not json"
        );
    }

    #[test]
    fn missing_theme_files_mean_defaults() {
        let dir = TempDir::new("theme-none");
        assert!(load_theme_settings_from(dir.path()).is_none());
        // Nothing is created when there is nothing to migrate.
        assert!(!dir.path().join("theme.json").exists());
    }

    #[test]
    fn v1_3_0_settings_bin_migrates_and_is_left_untouched() {
        let dir = TempDir::new("settings-migrate");
        std::fs::write(dir.path().join("settings.bin"), SETTINGS_V130).unwrap();

        let stored = load_settings_from(dir.path());
        assert_eq!(stored.theme.as_deref(), Some("light"));
        assert_eq!(stored.settings.language, "ja-JP");
        assert_eq!(stored.settings.directory_settings.len(), 1);
        assert_eq!(
            std::fs::read(dir.path().join("settings.bin")).unwrap(),
            SETTINGS_V130
        );

        let reloaded = load_settings_from(dir.path());
        assert_eq!(
            serde_json::to_value(&reloaded.settings).unwrap(),
            serde_json::to_value(&stored.settings).unwrap()
        );
        assert_eq!(reloaded.theme, stored.theme);
    }

    #[test]
    fn global_sort_survives_save_and_old_files_default() {
        use crate::gui::utils::SortColumn;
        use crate::gui::utils::SortKey;
        let dir = TempDir::new("settings-sort");
        let mut stored = load_settings_from(dir.path());
        assert!(stored.settings.sort_keys.is_empty());
        stored.settings.sort_keys = vec![SortKey {
            column: SortColumn::Size,
            ascending: false,
        }];
        save_settings_to(dir.path(), &stored).unwrap();
        let loaded = load_settings_from(dir.path());
        assert_eq!(loaded.settings.sort_keys.len(), 1);
        assert_eq!(loaded.settings.sort_keys[0].column, SortColumn::Size);
        assert!(!loaded.settings.sort_keys[0].ascending);

        // A file written before `sort_keys` existed still loads.
        let merged = from_value_with_defaults(
            &AppSettings::default(),
            serde_json::json!({ "sort_column": "Size", "sort_ascending": false }),
        );
        assert!(merged.value.sort_keys.is_empty());
        assert!(!merged.value.sort_ascending);
    }

    #[test]
    fn settings_round_trip_and_defaults() {
        let dir = TempDir::new("settings-roundtrip");
        let defaults = load_settings_from(dir.path());
        assert_eq!(
            defaults.settings.start_path,
            Some(PathBuf::from(MY_PC_PATH))
        );

        let mut stored = defaults.clone();
        stored.settings.time_format_24h = true;
        stored.settings.window_size_mode = WindowSizeMode::FullScreen;
        stored.theme = Some("dark".into());
        save_settings_to(dir.path(), &stored).unwrap();

        let loaded = load_settings_from(dir.path());
        assert!(loaded.settings.time_format_24h);
        assert!(matches!(
            loaded.settings.window_size_mode,
            WindowSizeMode::FullScreen
        ));
        assert_eq!(loaded.theme.as_deref(), Some("dark"));
    }

    #[test]
    fn settings_json_missing_fields_get_defaults() {
        let dir = TempDir::new("settings-partial");
        let json = r#"{"format":"eden-settings","version":2,"settings":{"time_format_24h":true}}"#;
        std::fs::write(dir.path().join("settings.json"), json).unwrap();
        let loaded = load_settings_from(dir.path());
        assert!(loaded.settings.time_format_24h);
        assert!(loaded.settings.folder_scanning_enabled);
        assert_eq!(loaded.theme, None);
    }

    #[test]
    fn v1_3_0_favorites_migrate_to_quick_access() {
        let dir = TempDir::new("favorites");
        assert_eq!(load_quick_access_from(dir.path()), None);

        std::fs::create_dir_all(dir.path().join("favorites")).unwrap();
        std::fs::write(
            dir.path().join("favorites").join("drive_C.bin"),
            FAVORITES_V130,
        )
        .unwrap();
        let items = load_quick_access_from(dir.path()).unwrap();
        assert_eq!(
            items,
            vec![
                QuickAccessEntry {
                    path: PathBuf::from("C:\\Users\\Andrew\\Desktop"),
                    label: None
                },
                QuickAccessEntry {
                    path: PathBuf::from("G:\\projects"),
                    label: None
                },
            ]
        );
        assert_eq!(
            std::fs::read(dir.path().join("favorites").join("drive_C.bin")).unwrap(),
            FAVORITES_V130
        );

        let renamed = vec![QuickAccessEntry {
            path: PathBuf::from("G:\\projects"),
            label: Some("Code".into()),
        }];
        save_quick_access_to(dir.path(), &renamed).unwrap();
        assert_eq!(load_quick_access_from(dir.path()).unwrap(), renamed);

        // An explicitly emptied list stays empty instead of re-seeding.
        save_quick_access_to(dir.path(), &[]).unwrap();
        assert_eq!(load_quick_access_from(dir.path()), Some(Vec::new()));
    }
}
