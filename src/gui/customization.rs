//! Layout preferences (toolbar, status bar, sidebar). Stored in
//! `settings.json` under `ui`; every field has a default, so older files load.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolbarLayout {
    /// Buttons share the row with the address bar (EdenExplorer's classic look).
    #[default]
    SingleRow,
    /// A separate command bar under the address bar, like Explorer.
    CommandBar,
}

impl ToolbarLayout {
    pub const ALL: [ToolbarLayout; 2] = [ToolbarLayout::SingleRow, ToolbarLayout::CommandBar];

    pub fn label_key(self) -> &'static str {
        match self {
            ToolbarLayout::SingleRow => "settings_toolbar_single_row",
            ToolbarLayout::CommandBar => "settings_toolbar_command_bar",
        }
    }
}

/// Which toolbar buttons are shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ToolbarPrefs {
    pub layout: ToolbarLayout,
    /// Show text next to the icons.
    pub labels: bool,
    pub new_folder: bool,
    pub new_file: bool,
    /// Cut, Copy, Paste, Rename, Delete, Properties.
    pub edit_buttons: bool,
    pub sort_menu: bool,
    pub view_menu: bool,
    pub favorite: bool,
    pub terminal: bool,
}

impl Default for ToolbarPrefs {
    /// Matches the toolbar of v1.3.0.
    fn default() -> Self {
        Self {
            layout: ToolbarLayout::SingleRow,
            labels: false,
            new_folder: true,
            new_file: true,
            edit_buttons: false,
            sort_menu: false,
            view_menu: true,
            favorite: true,
            terminal: true,
        }
    }
}

impl ToolbarPrefs {
    /// Explorer-like command bar: New, edit buttons, Sort, View with labels.
    pub fn explorer_like() -> Self {
        Self {
            layout: ToolbarLayout::CommandBar,
            labels: true,
            new_folder: true,
            new_file: true,
            edit_buttons: true,
            sort_menu: true,
            view_menu: true,
            favorite: true,
            terminal: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolbarItem {
    NewFolder,
    NewFile,
    Cut,
    Copy,
    Paste,
    Rename,
    Delete,
    Properties,
    Sort,
    View,
    Favorite,
    Terminal,
    Separator,
}

impl ToolbarItem {
    /// Label shown with `labels` on (i18n key).
    pub fn label_key(self) -> &'static str {
        match self {
            ToolbarItem::NewFolder => "toolbar_new_folder",
            ToolbarItem::NewFile => "toolbar_new_file",
            ToolbarItem::Cut => "inputs_cut",
            ToolbarItem::Copy => "inputs_copy",
            ToolbarItem::Paste => "inputs_paste",
            ToolbarItem::Rename => "inputs_rename",
            ToolbarItem::Delete => "inputs_delete",
            ToolbarItem::Properties => "properties",
            ToolbarItem::Sort => "toolbar_sort",
            ToolbarItem::View => "toolbar_view",
            ToolbarItem::Favorite => "toolbar_favorite",
            ToolbarItem::Terminal => "toolbar_terminal",
            ToolbarItem::Separator => "",
        }
    }

    /// Explorer shows edit buttons as icons only even with labels on.
    pub fn shows_label(self, labels: bool) -> bool {
        labels
            && !matches!(
                self,
                ToolbarItem::Cut
                    | ToolbarItem::Copy
                    | ToolbarItem::Paste
                    | ToolbarItem::Rename
                    | ToolbarItem::Delete
                    | ToolbarItem::Properties
                    | ToolbarItem::Separator
            )
    }
}

/// The toolbar buttons in display order, with separators between non-empty
/// groups. `extra_actions_bar` is the older setting that also turns on the
/// edit buttons.
pub fn toolbar_items(prefs: &ToolbarPrefs, extra_actions_bar: bool) -> Vec<ToolbarItem> {
    let edit = prefs.edit_buttons || extra_actions_bar;
    let groups: [Vec<ToolbarItem>; 4] = [
        [
            (prefs.new_folder, ToolbarItem::NewFolder),
            (prefs.new_file, ToolbarItem::NewFile),
        ]
        .into_iter()
        .filter_map(|(on, item)| on.then_some(item))
        .collect(),
        if edit {
            vec![
                ToolbarItem::Cut,
                ToolbarItem::Copy,
                ToolbarItem::Paste,
                ToolbarItem::Rename,
                ToolbarItem::Delete,
                ToolbarItem::Properties,
            ]
        } else {
            Vec::new()
        },
        [
            (prefs.sort_menu, ToolbarItem::Sort),
            (prefs.view_menu, ToolbarItem::View),
        ]
        .into_iter()
        .filter_map(|(on, item)| on.then_some(item))
        .collect(),
        [
            (prefs.favorite, ToolbarItem::Favorite),
            (prefs.terminal, ToolbarItem::Terminal),
        ]
        .into_iter()
        .filter_map(|(on, item)| on.then_some(item))
        .collect(),
    ];

    let mut items = Vec::new();
    for group in groups.into_iter().filter(|g| !g.is_empty()) {
        if !items.is_empty() {
            items.push(ToolbarItem::Separator);
        }
        items.extend(group);
    }
    items
}

/// Whether a toolbar button is usable in the current view.
#[derive(Clone, Copy, Debug, Default)]
pub struct ToolbarState {
    pub is_root: bool,
    pub is_recycle_bin: bool,
    pub has_selection: bool,
    pub clipboard_has_files: bool,
}

pub fn toolbar_item_enabled(item: ToolbarItem, s: ToolbarState) -> bool {
    let normal_folder = !s.is_root && !s.is_recycle_bin;
    match item {
        ToolbarItem::NewFolder | ToolbarItem::NewFile => !s.is_recycle_bin,
        ToolbarItem::Cut | ToolbarItem::Copy | ToolbarItem::Rename => {
            s.has_selection && !s.is_recycle_bin
        }
        ToolbarItem::Paste => normal_folder && s.clipboard_has_files,
        ToolbarItem::Delete | ToolbarItem::Properties => s.has_selection,
        ToolbarItem::Sort => true,
        ToolbarItem::View => normal_folder,
        ToolbarItem::Favorite => normal_folder,
        ToolbarItem::Terminal => !s.is_recycle_bin,
        ToolbarItem::Separator => false,
    }
}

/// What the status bar under each pane shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct StatusBarPrefs {
    pub visible: bool,
    /// "13 items" (and the selection) on the left, like Explorer.
    pub item_count: bool,
    /// Total size of the selected files.
    pub selection_size: bool,
    /// Details / Gallery buttons at the right edge.
    pub view_toggles: bool,
    /// The size / folder / file counters of v1.3.0.
    pub counters: bool,
}

impl Default for StatusBarPrefs {
    /// Matches v1.3.0.
    fn default() -> Self {
        Self {
            visible: true,
            item_count: false,
            selection_size: false,
            view_toggles: false,
            counters: true,
        }
    }
}

impl StatusBarPrefs {
    pub fn explorer_like() -> Self {
        Self {
            visible: true,
            item_count: true,
            selection_size: true,
            view_toggles: true,
            counters: false,
        }
    }
}

/// Numbers shown in the status bar for the visible (filtered) items.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StatusStats {
    pub items: usize,
    pub selected: usize,
    /// Bytes of the selected files (folders are not counted).
    pub selected_bytes: u64,
    pub selected_files: usize,
}

/// Left status text: "13 items", plus "| 2 items selected 4.2 MB" when there
/// is a selection. `item`/`items`/`selected` are the translated words and
/// `format_size` turns bytes into text.
pub fn status_left_text(
    prefs: &StatusBarPrefs,
    stats: StatusStats,
    item: &str,
    items: &str,
    selected: &str,
    format_size: impl Fn(u64) -> String,
) -> String {
    let noun = |n: usize| if n == 1 { item } else { items };
    let mut text = String::new();
    if prefs.item_count {
        text = format!("{} {}", stats.items, noun(stats.items));
        if stats.selected > 0 {
            text.push_str(&format!(
                "    |    {} {} {}",
                stats.selected,
                noun(stats.selected),
                selected
            ));
        }
    }
    if prefs.selection_size && stats.selected_files > 0 {
        if !text.is_empty() {
            text.push_str("  ");
        }
        text.push_str(&format_size(stats.selected_bytes));
    }
    text
}

/// Everything under `ui` in `settings.json`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiPreferences {
    pub toolbar: ToolbarPrefs,
    pub status_bar: StatusBarPrefs,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ToolbarItem::*;

    #[test]
    fn default_toolbar_matches_v1_3_0() {
        assert_eq!(
            toolbar_items(&ToolbarPrefs::default(), false),
            vec![
                NewFolder, NewFile, Separator, View, Separator, Favorite, Terminal
            ]
        );
    }

    #[test]
    fn extra_actions_setting_still_adds_edit_buttons() {
        let items = toolbar_items(&ToolbarPrefs::default(), true);
        assert_eq!(
            &items[..9],
            &[
                NewFolder, NewFile, Separator, Cut, Copy, Paste, Rename, Delete, Properties
            ]
        );
    }

    #[test]
    fn separators_only_between_non_empty_groups() {
        let prefs = ToolbarPrefs {
            new_folder: false,
            new_file: false,
            view_menu: false,
            favorite: false,
            ..ToolbarPrefs::default()
        };
        assert_eq!(toolbar_items(&prefs, false), vec![Terminal]);
        let none = ToolbarPrefs {
            terminal: false,
            ..prefs
        };
        assert!(toolbar_items(&none, false).is_empty());
    }

    #[test]
    fn explorer_like_has_labels_and_menus() {
        let prefs = ToolbarPrefs::explorer_like();
        let items = toolbar_items(&prefs, false);
        assert!(items.contains(&Sort) && items.contains(&View) && items.contains(&Cut));
        assert!(NewFolder.shows_label(prefs.labels));
        assert!(!Cut.shows_label(prefs.labels));
        assert!(!NewFolder.shows_label(false));
    }

    #[test]
    fn enabled_states() {
        let s = ToolbarState {
            has_selection: true,
            clipboard_has_files: true,
            ..Default::default()
        };
        assert!(toolbar_item_enabled(Cut, s));
        assert!(toolbar_item_enabled(Paste, s));
        let bin = ToolbarState {
            is_recycle_bin: true,
            ..s
        };
        assert!(!toolbar_item_enabled(Cut, bin));
        assert!(!toolbar_item_enabled(NewFolder, bin));
        assert!(toolbar_item_enabled(Delete, bin));
        let pc = ToolbarState {
            is_root: true,
            ..Default::default()
        };
        assert!(!toolbar_item_enabled(View, pc));
        assert!(!toolbar_item_enabled(Copy, pc));
    }

    #[test]
    fn status_text() {
        let size = |b: u64| format!("{b} B");
        let explorer = StatusBarPrefs::explorer_like();
        let stats = StatusStats {
            items: 13,
            ..Default::default()
        };
        assert_eq!(
            status_left_text(&explorer, stats, "item", "items", "selected", size),
            "13 items"
        );
        let stats = StatusStats {
            items: 1,
            selected: 1,
            selected_bytes: 42,
            selected_files: 1,
        };
        assert_eq!(
            status_left_text(&explorer, stats, "item", "items", "selected", size),
            "1 item    |    1 item selected  42 B"
        );
        // Only folders selected: no size.
        let stats = StatusStats {
            items: 3,
            selected: 2,
            selected_bytes: 0,
            selected_files: 0,
        };
        assert_eq!(
            status_left_text(&explorer, stats, "item", "items", "selected", size),
            "3 items    |    2 items selected"
        );
        // v1.3.0 defaults show nothing on the left.
        assert_eq!(
            status_left_text(&StatusBarPrefs::default(), stats, "a", "b", "c", size),
            ""
        );
    }

    #[test]
    fn preferences_load_with_missing_fields() {
        let prefs: UiPreferences = serde_json::from_str(r#"{"toolbar":{"labels":true}}"#).unwrap();
        assert!(prefs.toolbar.labels);
        assert!(prefs.toolbar.new_folder);
        let prefs: UiPreferences = serde_json::from_str("{}").unwrap();
        assert_eq!(prefs, UiPreferences::default());
    }
}
