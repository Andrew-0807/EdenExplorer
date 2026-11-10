use crate::core::i18n::I18n;
use crate::core::{
    fs::{DateStyle, MY_PC_PATH},
    indexer::{
        WindowSizeMode, default_item_viewer_drive_column_size,
        default_item_viewer_file_column_size, default_recycle_bin_column_size,
    },
    utils::text::apply_eden_text_overrides,
    utils::widgets::{
        apply_eden_visual_overrides, draw_checkbox_sized, draw_dropdown_sized, eden_button,
        eden_icon_button, eden_text_label, eden_toggle_button,
    },
};
use crate::gui::customization::{
    CheckboxVisibility, SidebarPrefs, StatusBarPrefs, ToolbarLayout, ToolbarPrefs,
};
use crate::gui::icon_system::{AppIcon, draw_icon, paint_icon};
use crate::gui::theme::ThemePalette;
use crate::gui::utils::SortColumn;
use crate::gui::windows::containers::enums::ItemViewerHeaderColumn;
use crate::gui::windows::enums::SettingsAction;
use crate::gui::windows::structs::{AppSettings, SettingsTab, SettingsWindow};
use crate::gui::windows::windowsoverrides::clamp_dialog_size;
use eframe::egui;
use egui::RichText;
use std::path::PathBuf;

/// Minimum height of one settings row, so rows with a 1-line and a 2-line label line up.
const SETTINGS_ROW_HEIGHT: f32 = 40.0;
/// Vertical gap between rows.
const SETTINGS_ROW_GAP: f32 = 2.0;
/// Space between the label column and the control column.
const SETTINGS_CONTROL_GAP: f32 = 12.0;
const SETTINGS_COMBO_WIDTH: f32 = 200.0;
const SETTINGS_VALUE_WIDTH: f32 = 110.0;
/// Height of combo buttons, drag values and push buttons.
const SETTINGS_CONTROL_HEIGHT: f32 = 32.0;
/// Click target of a checkbox and its drawn box size.
const SETTINGS_CHECKBOX_HIT: f32 = 30.0;
const SETTINGS_CHECKBOX_SIZE: f32 = 20.0;
/// Square size of icon buttons.
const SETTINGS_ICON_BUTTON: f32 = 32.0;
/// Chip column of the shortcuts list; fits the longest binding.
const SETTINGS_SHORTCUT_WIDTH: f32 = 220.0;
const SETTINGS_WINDOW_DEFAULT_SIZE: [f32; 2] = [820.0, 640.0];
const SETTINGS_WINDOW_MIN_SIZE: [f32; 2] = [640.0, 480.0];

/// Text size of settings labels and controls. The rest of the app keeps `palette.text_size`.
fn body_size(palette: &ThemePalette) -> f32 {
    palette.text_size + 4.0
}

const HOTKEYS: &[(&str, &str)] = &[
    ("F1", "hotkey_toggle_fullscreen"),
    ("Ctrl+Shift+C", "hotkey_copy_selected_path"),
    ("Ctrl+W", "hotkey_close_current_tab"),
    ("Ctrl+T", "hotkey_new_tab"),
    ("Ctrl+Tab", "hotkey_next_tab"),
    ("Ctrl+Shift+Tab", "hotkey_previous_tab"),
    ("Ctrl+Shift+N", "hotkey_new_folder"),
    ("Ctrl+R (F5)", "hotkey_refresh_directory"),
    ("Alt+D", "hotkey_select_address_bar"),
    ("F2", "hotkey_rename_file"),
    ("Alt+Enter", "hotkey_open_properties"),
    ("Home / End", "hotkey_scroll_and_select_edges"),
    ("Alt+Left Arrow (Backspace)", "hotkey_navigate_back"),
    ("Alt+Right Arrow", "hotkey_navigate_forward"),
    ("Alt+Up Arrow", "hotkey_navigate_up"),
    ("Enter", "hotkey_run_default_program"),
    ("Ctrl+Delete", "hotkey_permanent_delete"),
];

const HOTKEY_GROUPS: &[(&str, &[usize])] = &[
    ("hotkey_group_application", &[0]),
    ("hotkey_group_tabs", &[2, 3, 4, 5]),
    ("hotkey_group_files_folders", &[1, 6, 9, 10, 15, 16]),
    ("hotkey_group_navigation", &[7, 8, 11, 12, 13, 14]),
];

/// Returns the last `max_chars` characters of `s` prefixed with "...", or `s`
/// unchanged when it already fits. Counts chars, not bytes, so multi-byte
/// paths (CJK, accented) never get sliced inside a code point.
fn truncate_path_tail(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count > max_chars {
        let tail: String = s.chars().skip(char_count - max_chars).collect();
        format!("...{tail}")
    } else {
        s.to_string()
    }
}

// Read-only list of the bindings hard-coded in the app (`handle_global_shortcuts`, item viewer
// keyboard handling). Keep in sync when a binding changes.
fn draw_hotkeys_tab(ui: &mut egui::Ui, palette: &ThemePalette, i18n: &I18n) {
    for (group_name, indices) in HOTKEY_GROUPS {
        settings_heading(ui, palette, &i18n.tr(group_name));
        settings_group(ui, |ui| {
            for index in *indices {
                let (shortcut, description_key) = HOTKEYS[*index];
                let description = i18n.tr(description_key);

                setting_row(
                    ui,
                    palette,
                    &description,
                    None,
                    SETTINGS_SHORTCUT_WIDTH,
                    |ui| {
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(SETTINGS_SHORTCUT_WIDTH, 28.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_stroke(
                            rect,
                            4.0,
                            egui::Stroke::new(1.0, palette.borders_default),
                            egui::StrokeKind::Inside,
                        );
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            shortcut,
                            egui::FontId::proportional(body_size(palette) - 1.0),
                            palette.primary,
                        );
                    },
                );
            }
        });
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            folder_scanning_enabled: true,
            show_hidden_files_folders: true,
            show_item_viewer_icons: true,
            windows_context_menu_enabled: false,
            extra_actions_bar: false,
            start_path: Some(PathBuf::from(MY_PC_PATH)),
            window_size_mode: WindowSizeMode::default(),
            pinned_tabs: Vec::new(),
            remember_open_tabs: false,
            unremembered_tabs: Vec::new(),
            remembered_active_tab: None,
            time_format_24h: false,
            date_style: DateStyle::default(),
            sort_column: SortColumn::Name,
            sort_ascending: true,
            sort_keys: Vec::new(),
            language: "en-US".to_string(),
            item_viewer_file_column_order: vec![
                ItemViewerHeaderColumn::Type,
                ItemViewerHeaderColumn::Size,
                ItemViewerHeaderColumn::Tags,
                ItemViewerHeaderColumn::Modified,
                ItemViewerHeaderColumn::Created,
            ],
            item_viewer_drive_column_order: vec![
                ItemViewerHeaderColumn::Type,
                ItemViewerHeaderColumn::Size,
                ItemViewerHeaderColumn::Usage,
            ],
            recycle_bin_column_order: vec![
                ItemViewerHeaderColumn::OriginalDirectory,
                ItemViewerHeaderColumn::Type,
                ItemViewerHeaderColumn::Size,
                ItemViewerHeaderColumn::Deleted,
                ItemViewerHeaderColumn::Created,
            ],
            item_viewer_file_column_sizes: default_item_viewer_file_column_size(),
            item_viewer_drive_column_sizes: default_item_viewer_drive_column_size(),
            recycle_bin_column_sizes: default_recycle_bin_column_size(),
            directory_settings: vec![],
            ui: Default::default(),
        }
    }
}

const DEFAULT_FILE_COLUMNS: [ItemViewerHeaderColumn; 5] = [
    ItemViewerHeaderColumn::Type,
    ItemViewerHeaderColumn::Size,
    ItemViewerHeaderColumn::Tags,
    ItemViewerHeaderColumn::Modified,
    ItemViewerHeaderColumn::Created,
];

fn set_default_file_column_visible(
    order: &mut Vec<ItemViewerHeaderColumn>,
    column: ItemViewerHeaderColumn,
    visible: bool,
) {
    if visible {
        if order.contains(&column) {
            return;
        }

        let default_index = DEFAULT_FILE_COLUMNS
            .iter()
            .position(|candidate| *candidate == column)
            .unwrap_or(order.len());
        let insert_index = order
            .iter()
            .filter_map(|existing| DEFAULT_FILE_COLUMNS.iter().position(|c| c == existing))
            .filter(|index| *index < default_index)
            .count();
        order.insert(insert_index, column);
    } else {
        order.retain(|existing| *existing != column);
    }
}

fn file_column_label(i18n: &I18n, column: ItemViewerHeaderColumn) -> String {
    match column {
        ItemViewerHeaderColumn::Name => i18n.tr("explorer_cols_name"),
        ItemViewerHeaderColumn::Type => i18n.tr("explorer_cols_type"),
        ItemViewerHeaderColumn::Size => i18n.tr("explorer_cols_size"),
        ItemViewerHeaderColumn::Tags => i18n.tr("explorer_cols_tags"),
        ItemViewerHeaderColumn::Modified => i18n.tr("explorer_cols_modified"),
        ItemViewerHeaderColumn::Created => i18n.tr("explorer_cols_created"),
        _ => String::new(),
    }
}

fn draw_view_defaults_tab(
    ui: &mut egui::Ui,
    settings: &mut SettingsWindow,
    palette: &ThemePalette,
    i18n: &I18n,
) -> Option<SettingsAction> {
    let mut changed = false;
    let current_order = settings
        .current_settings
        .item_viewer_file_column_order
        .clone();

    if settings_button(ui, palette, &i18n.tr("settings_clear_directory_cache")).clicked() {
        settings.show_clear_directory_cache_confirmation = true;
    }

    settings_heading(ui, palette, &i18n.tr("settings_view_defaults_columns"));
    settings_group(ui, |ui| {
        for column in DEFAULT_FILE_COLUMNS {
            let mut visible = current_order.contains(&column);
            if setting_checkbox(
                ui,
                palette,
                &mut visible,
                RichText::new(file_column_label(i18n, column)),
                format!("view_default_column_{column:?}"),
            ) {
                set_default_file_column_visible(
                    &mut settings.current_settings.item_viewer_file_column_order,
                    column,
                    visible,
                );
                changed = true;
            }
        }
    });

    settings_heading(ui, palette, &i18n.tr("settings_default_column_width"));
    settings_group(ui, |ui| {
        let defaults = default_item_viewer_file_column_size();
        if settings
            .current_settings
            .item_viewer_file_column_sizes
            .len()
            != defaults.len()
        {
            settings.current_settings.item_viewer_file_column_sizes = defaults;
        }

        for (index, column) in [
            ItemViewerHeaderColumn::Name,
            ItemViewerHeaderColumn::Type,
            ItemViewerHeaderColumn::Size,
            ItemViewerHeaderColumn::Tags,
            ItemViewerHeaderColumn::Modified,
            ItemViewerHeaderColumn::Created,
        ]
        .into_iter()
        .enumerate()
        {
            let mut width = settings.current_settings.item_viewer_file_column_sizes[index];
            setting_row(
                ui,
                palette,
                &file_column_label(i18n, column),
                None,
                SETTINGS_VALUE_WIDTH,
                |ui| {
                    if ui
                        .add_sized(
                            [SETTINGS_VALUE_WIDTH, SETTINGS_CONTROL_HEIGHT],
                            egui::DragValue::new(&mut width)
                                .range(20.0..=1000.0)
                                .speed(1.0),
                        )
                        .changed()
                    {
                        settings.current_settings.item_viewer_file_column_sizes[index] = width;
                        changed = true;
                    }
                },
            );
        }
    });

    changed.then_some(SettingsAction::ApplySettings)
}

/// A checkbox row bound to one preference; true when it changed.
fn pref_checkbox(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    i18n: &I18n,
    key: &str,
    value: &mut bool,
) -> bool {
    setting_checkbox(
        ui,
        palette,
        value,
        RichText::new(i18n.tr(key)),
        ("appearance", key),
    )
}

fn draw_appearance_tab(
    ui: &mut egui::Ui,
    settings: &mut SettingsWindow,
    palette: &ThemePalette,
    i18n: &I18n,
) -> Option<SettingsAction> {
    let mut changed = false;
    let prefs = &mut settings.current_settings.ui;

    settings_heading(ui, palette, &i18n.tr("settings_theme_group"));
    settings_group(ui, |ui| {
        if pref_checkbox(
            ui,
            palette,
            i18n,
            "settings_theme_follow_system",
            &mut prefs.theme_follow_system,
        ) {
            crate::core::system_theme::mark_system_theme_dirty();
            changed = true;
        }
        setting_row(
            ui,
            palette,
            &i18n.tr("settings_zoom"),
            None,
            SETTINGS_VALUE_WIDTH,
            |ui| {
                let mut percent = (prefs.zoom * 100.0).round();
                if ui
                    .add_sized(
                        [SETTINGS_VALUE_WIDTH, SETTINGS_CONTROL_HEIGHT],
                        egui::DragValue::new(&mut percent)
                            .range(50.0..=300.0)
                            .speed(1.0)
                            .suffix("%"),
                    )
                    .changed()
                {
                    prefs.zoom = percent / 100.0;
                    changed = true;
                }
            },
        );
    });

    settings_heading(ui, palette, &i18n.tr("settings_item_list"));
    settings_group(ui, |ui| {
        setting_row(
            ui,
            palette,
            &i18n.tr("settings_checkboxes"),
            None,
            SETTINGS_COMBO_WIDTH,
            |ui| {
                let current = i18n.tr(prefs.item_checkboxes.label_key());
                settings_dropdown(
                    ui,
                    palette,
                    "settings_checkboxes",
                    SETTINGS_COMBO_WIDTH,
                    current,
                    |ui| {
                        for mode in CheckboxVisibility::ALL {
                            if ui
                                .selectable_label(
                                    prefs.item_checkboxes == mode,
                                    i18n.tr(mode.label_key()),
                                )
                                .clicked()
                            {
                                prefs.item_checkboxes = mode;
                                changed = true;
                            }
                        }
                    },
                );
            },
        );
    });

    settings_heading(ui, palette, &i18n.tr("settings_toolbar"));
    settings_group(ui, |ui| {
        setting_row(
            ui,
            palette,
            &i18n.tr("settings_toolbar_layout"),
            None,
            SETTINGS_COMBO_WIDTH,
            |ui| {
                let current = i18n.tr(prefs.toolbar.layout.label_key());
                settings_dropdown(
                    ui,
                    palette,
                    "settings_toolbar_layout",
                    SETTINGS_COMBO_WIDTH,
                    current,
                    |ui| {
                        for layout in ToolbarLayout::ALL {
                            if ui
                                .selectable_label(
                                    prefs.toolbar.layout == layout,
                                    i18n.tr(layout.label_key()),
                                )
                                .clicked()
                            {
                                prefs.toolbar.layout = layout;
                                changed = true;
                            }
                        }
                    },
                );
            },
        );
        let t = &mut prefs.toolbar;
        for (key, value) in [
            ("settings_toolbar_labels", &mut t.labels),
            ("settings_toolbar_new_folder", &mut t.new_folder),
            ("settings_toolbar_new_file", &mut t.new_file),
            ("settings_toolbar_edit_buttons", &mut t.edit_buttons),
            ("settings_toolbar_sort", &mut t.sort_menu),
            ("settings_toolbar_view", &mut t.view_menu),
            ("settings_toolbar_favorite", &mut t.favorite),
            ("settings_toolbar_terminal", &mut t.terminal),
        ] {
            changed |= pref_checkbox(ui, palette, i18n, key, value);
        }
    });

    settings_heading(ui, palette, &i18n.tr("settings_status_bar"));
    settings_group(ui, |ui| {
        let s = &mut prefs.status_bar;
        for (key, value) in [
            ("settings_status_bar_visible", &mut s.visible),
            ("settings_status_bar_item_count", &mut s.item_count),
            ("settings_status_bar_selection_size", &mut s.selection_size),
            ("settings_status_bar_view_toggles", &mut s.view_toggles),
            ("settings_status_bar_counters", &mut s.counters),
        ] {
            changed |= pref_checkbox(ui, palette, i18n, key, value);
        }
    });

    settings_heading(ui, palette, &i18n.tr("settings_sidebar"));
    settings_group(ui, |ui| {
        let s = &mut prefs.sidebar;
        for (key, value) in [
            ("settings_sidebar_places", &mut s.places),
            ("settings_sidebar_quick_access", &mut s.quick_access),
            ("settings_sidebar_drives", &mut s.drives),
            ("settings_sidebar_network", &mut s.network),
            ("settings_sidebar_linux", &mut s.linux),
            ("settings_sidebar_pin_glyphs", &mut s.pin_glyphs),
            ("settings_sidebar_usage_bars", &mut s.drive_usage_bars),
            ("settings_sidebar_titles", &mut s.section_titles),
        ] {
            changed |= pref_checkbox(ui, palette, i18n, key, value);
        }
    });

    ui.add_space(8.0);
    if settings_button(ui, palette, &i18n.tr("settings_toolbar_explorer_like")).clicked() {
        prefs.toolbar = ToolbarPrefs::explorer_like();
        prefs.status_bar = StatusBarPrefs::explorer_like();
        prefs.sidebar = SidebarPrefs::explorer_like();
        changed = true;
    }

    changed.then_some(SettingsAction::ApplySettings)
}

fn draw_general_tab(
    ui: &mut egui::Ui,
    settings: &mut SettingsWindow,
    palette: &ThemePalette,
    i18n: &mut I18n,
) -> Option<SettingsAction> {
    let mut action = None;

    if settings_button(ui, palette, &i18n.tr("settings_reset")).clicked() {
        settings.show_reset_confirmation = true;
    }

    settings_group(ui, |ui| {
        let language_label = i18n.tr("language");
        let english = i18n.tr("english");
        let japanese = i18n.tr("japanese");
        let indonesian = i18n.tr("indonesian");
        let chinese_simple = i18n.tr("chinese_simple");
        let chinese_traditional = i18n.tr("chinese_traditional");
        let chinese_hk = i18n.tr("chinese_traditional_hk");

        setting_row(
            ui,
            palette,
            &language_label,
            None,
            SETTINGS_COMBO_WIDTH,
            |ui| {
                let mut selected_locale = settings.current_settings.language.clone();

                if combo_box_string(
                    ui,
                    palette,
                    "language_selector",
                    SETTINGS_COMBO_WIDTH,
                    &mut selected_locale,
                    &[
                        ("en-US", english.clone()),
                        ("ja-JP", japanese.clone()),
                        ("id-ID", indonesian.clone()),
                        ("zh-CN", chinese_simple.clone()),
                        ("zh-TW", chinese_traditional.clone()),
                        ("zh-HK", chinese_hk.clone()),
                    ],
                ) {
                    i18n.set_locale(&selected_locale);
                    settings.current_settings.language = selected_locale;
                    action = Some(SettingsAction::ApplySettings);
                }
            },
        );

        // Folder Scanning
        if setting_toggle(
            ui,
            palette,
            &mut settings.current_settings.folder_scanning_enabled,
            &i18n.tr("settings_directoryscanning"),
            Some(&i18n.tr("tooltip_settings_directoryscanning")),
            "settings_directoryscanning",
        ) {
            // Auto-save when setting changes
            action = Some(SettingsAction::ApplySettings);
        }
        // Hidden Files/Folders
        if setting_toggle(
            ui,
            palette,
            &mut settings.current_settings.show_hidden_files_folders,
            &i18n.tr("settings_show_hidden_files_folders"),
            Some(&i18n.tr("tooltip_settings_show_hidden_files_folders")),
            "settings_show_hidden_files_folders",
        ) {
            // Auto-save when setting changes
            action = Some(SettingsAction::ApplySettings);
        }
        // Remember Open Tabs
        if setting_toggle(
            ui,
            palette,
            &mut settings.current_settings.remember_open_tabs,
            &i18n.tr("settings_remember_open_tabs"),
            Some(&i18n.tr("tooltip_settings_remember_open_tabs")),
            "settings_remember_open_tabs",
        ) {
            if settings.current_settings.remember_open_tabs {
                // Enabling the global setting takes precedence over any
                // per-tab opt-outs that were saved previously.
                settings.current_settings.unremembered_tabs.clear();
            }
            action = Some(SettingsAction::ApplySettings);
        }
        // Show/Hide Item Viewer File Icons
        if setting_checkbox(
            ui,
            palette,
            &mut settings.current_settings.show_item_viewer_icons,
            RichText::new(i18n.tr("settings_show_item_viewer_file_icons")),
            "settings_show_item_viewer_file_icons",
        ) {
            // Auto-save when setting changes
            action = Some(SettingsAction::ApplySettings);
        }
        if setting_toggle(
            ui,
            palette,
            &mut settings.current_settings.windows_context_menu_enabled,
            &i18n.tr("settings_contextmenu_enable"),
            Some(&i18n.tr("tooltip_settings_contextmenu")),
            "settings_contextmenu_enable",
        ) {
            action = Some(SettingsAction::ApplySettings);
        }
        if setting_checkbox(
            ui,
            palette,
            &mut settings.current_settings.extra_actions_bar,
            RichText::new(i18n.tr("settings_enable_extra_action_buttons_in_item_viewer")),
            "settings_enable_extra_action_buttons_in_item_viewer",
        ) {
            action = Some(SettingsAction::ApplySettings);
        }
        // Date Style Section
        setting_row(
            ui,
            palette,
            &i18n.tr("settings_datestyle"),
            Some(&i18n.tr("tooltip_settings_datestyle")),
            SETTINGS_COMBO_WIDTH,
            |ui| {
                let mut selected_style = settings.current_settings.date_style;

                settings_dropdown(
                    ui,
                    palette,
                    "date_style_selector",
                    SETTINGS_COMBO_WIDTH,
                    match selected_style {
                        DateStyle::Iso => i18n.tr("settings_datestyle_iso_label"),
                        DateStyle::UsShort => i18n.tr("settings_datestyle_us_short_label"),
                        DateStyle::Long => i18n.tr("settings_datestyle_long_label"),
                    },
                    |ui| {
                        ui.selectable_value(
                            &mut selected_style,
                            DateStyle::Iso,
                            i18n.tr("settings_datestyle_iso"),
                        );
                        ui.selectable_value(
                            &mut selected_style,
                            DateStyle::UsShort,
                            i18n.tr("settings_datestyle_us_short"),
                        );
                        ui.selectable_value(
                            &mut selected_style,
                            DateStyle::Long,
                            i18n.tr("settings_datestyle_long"),
                        );
                    },
                );

                if selected_style != settings.current_settings.date_style {
                    settings.current_settings.date_style = selected_style;
                    action = Some(SettingsAction::ApplySettings);
                }
            },
        );

        // Time Format Section
        if setting_toggle(
            ui,
            palette,
            &mut settings.current_settings.time_format_24h,
            &i18n.tr("settings_timeformat_24h"),
            Some(&i18n.tr("tooltip_settings_timeformat")),
            "settings_timeformat_24h",
        ) {
            action = Some(SettingsAction::ApplySettings);
        }
    });
    settings_group(ui, |ui| {
        // Starting Path
        setting_row(
            ui,
            palette,
            &i18n.tr("settings_startpath"),
            None,
            2.0 * SETTINGS_ICON_BUTTON + 8.0,
            |ui| {
                if settings_icon_button(
                    ui,
                    palette,
                    AppIcon::ArrowCounterClockwise.regular(),
                    &i18n.tr("settings_startpath_reset_hover"),
                )
                .clicked()
                {
                    settings.current_settings.start_path = Some(PathBuf::from(MY_PC_PATH));
                    action = Some(SettingsAction::ApplySettings);
                }

                let choose_clicked = settings_icon_button(
                    ui,
                    palette,
                    AppIcon::FolderOpen.regular(),
                    &i18n.tr("settings_startpath_choose_hover"),
                )
                .clicked();
                if choose_clicked && let Some(path) = rfd::FileDialog::new().pick_folder() {
                    settings.current_settings.start_path = Some(path);
                    action = Some(SettingsAction::ApplySettings);
                }
            },
        );

        let path_text = settings
            .current_settings
            .start_path
            .as_ref()
            .map(|p| {
                if p.as_os_str() == MY_PC_PATH {
                    return i18n.tr("settings_startpath_default");
                }

                truncate_path_tail(&p.to_string_lossy(), 40)
            })
            .unwrap_or_else(|| i18n.tr("settings_startpath_default"));

        ui.add(
            egui::Label::new(
                RichText::new(path_text)
                    .size(body_size(palette) - 1.0)
                    .color(palette.text_normal),
            )
            .truncate(),
        )
        .on_hover_text(
            settings
                .current_settings
                .start_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
        );
        ui.add_space(SETTINGS_ROW_GAP * 3.0);

        // Window Size Section
        let mut window_size_changed = false;

        setting_row(
            ui,
            palette,
            &i18n.tr("settings_windowsize"),
            None,
            SETTINGS_COMBO_WIDTH + SETTINGS_ICON_BUTTON + 8.0,
            |ui| {
                let is_fullscreen = matches!(
                    settings.current_settings.window_size_mode,
                    WindowSizeMode::FullScreen
                );

                settings_dropdown(
                    ui,
                    palette,
                    "window_size_mode_selector",
                    SETTINGS_COMBO_WIDTH,
                    if is_fullscreen {
                        i18n.tr("settings_windowsize_fullscreen")
                    } else {
                        i18n.tr("settings_windowsize_custom")
                    },
                    |ui| {
                        if ui
                            .selectable_label(!is_fullscreen, i18n.tr("settings_windowsize_custom"))
                            .clicked()
                            && is_fullscreen
                        {
                            settings.current_settings.window_size_mode = WindowSizeMode::Custom {
                                width: 1200.0,
                                height: 800.0,
                            };
                            window_size_changed = true;
                        }

                        if ui
                            .selectable_label(
                                is_fullscreen,
                                i18n.tr("settings_windowsize_fullscreen"),
                            )
                            .clicked()
                            && !is_fullscreen
                        {
                            settings.current_settings.window_size_mode = WindowSizeMode::FullScreen;
                            window_size_changed = true;
                        }
                    },
                );

                if settings_icon_button(
                    ui,
                    palette,
                    AppIcon::ArrowCounterClockwise.regular(),
                    &i18n.tr("settings_windowsize_reset_hover"),
                )
                .clicked()
                {
                    settings.current_settings.window_size_mode = WindowSizeMode::default();
                    window_size_changed = true;
                }
            },
        );

        if let WindowSizeMode::Custom { width, height } =
            &mut settings.current_settings.window_size_mode
        {
            setting_row(
                ui,
                palette,
                &i18n.tr("settings_windowsize_width"),
                None,
                SETTINGS_VALUE_WIDTH,
                |ui| {
                    window_size_changed |= ui
                        .add_sized(
                            [SETTINGS_VALUE_WIDTH, SETTINGS_CONTROL_HEIGHT],
                            egui::DragValue::new(width).range(800.0..=4000.0).speed(1.0),
                        )
                        .changed();
                },
            );

            setting_row(
                ui,
                palette,
                &i18n.tr("settings_windowsize_height"),
                None,
                SETTINGS_VALUE_WIDTH,
                |ui| {
                    window_size_changed |= ui
                        .add_sized(
                            [SETTINGS_VALUE_WIDTH, SETTINGS_CONTROL_HEIGHT],
                            egui::DragValue::new(height)
                                .range(600.0..=3000.0)
                                .speed(1.0),
                        )
                        .changed();
                },
            );
        }

        if window_size_changed {
            action = Some(SettingsAction::ApplySettings);
        }

        // Favorites Reset
        if settings_button(ui, palette, &i18n.tr("settings_favorites_reset"))
            .on_hover_text(
                egui::RichText::new(i18n.tr("tooltip_settings_favorites_reset"))
                    .size(palette.tooltip_text_size)
                    .color(palette.tooltip_text_color),
            )
            .clicked()
        {
            settings.show_reset_favorites_confirmation = true;
        }
    });

    action
}
fn draw_confirmation_dialog(
    ctx: &egui::Context,
    palette: &ThemePalette,
    i18n: &I18n,
    title: &str,
    message_lines: &[&str],
) -> Option<bool> {
    let mut result = None;
    let mut should_close = ctx.input(|i| i.key_pressed(egui::Key::Escape));

    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .fixed_size([400.0, 150.0])
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(
            egui::Frame::popup(&ctx.style_of(ctx.theme()))
                .corner_radius(egui::CornerRadius::same(palette.large_radius)),
        )
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                eden_text_label(ui, palette, title);
                for line in message_lines {
                    eden_text_label(ui, palette, line);
                }
                ui.add_space(20.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if eden_button(ui, palette, &i18n.tr("close")).clicked() {
                        should_close = true;
                    }
                    apply_eden_visual_overrides(ui, palette);
                    apply_eden_text_overrides(ui, palette);
                    if ui
                        .button(RichText::new(i18n.tr("reset")).color(palette.tab_close_hover))
                        .clicked()
                    {
                        result = Some(true);
                        should_close = true;
                    }
                });
            });
        });

    if should_close {
        result.or(Some(false))
    } else {
        None
    }
}

fn info_icon_size(palette: &ThemePalette) -> f32 {
    palette.text_size + 5.0
}

fn info_icon(ui: &mut egui::Ui, hover_text: &str, palette: &ThemePalette) -> egui::Response {
    let size = info_icon_size(palette);
    let resp = draw_icon(
        ui,
        AppIcon::Question.regular(),
        size,
        palette.text_normal,
        hover_text,
    );

    if resp.hovered() {
        paint_icon(
            ui,
            resp.rect,
            AppIcon::Question.regular(),
            size,
            palette.primary,
        );
    }

    if resp.hovered() {
        ui.ctx()
            .output_mut(|o| o.cursor_icon = egui::CursorIcon::Default);
        egui::containers::Area::new(ui.next_auto_id())
            .current_pos(resp.rect.right_top())
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(hover_text)
                                .size(palette.text_size)
                                .color(ui.visuals().text_color()),
                        );
                    });
            });
    }

    resp
}

/// Heading above a group of rows.
fn settings_heading(ui: &mut egui::Ui, palette: &ThemePalette, text: &str) {
    ui.add_space(12.0);
    ui.label(
        RichText::new(text)
            .size(body_size(palette) + 2.0)
            .strong()
            .color(palette.text_header_section),
    );
    ui.add_space(4.0);
}

/// Full-width container of related rows.
fn settings_group(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        add_contents(ui);
    });
}

fn settings_button(ui: &mut egui::Ui, palette: &ThemePalette, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(
            RichText::new(text)
                .size(body_size(palette))
                .color(palette.text_normal),
        )
        .min_size(egui::vec2(0.0, SETTINGS_CONTROL_HEIGHT)),
    )
}

fn settings_icon_button(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    icon: crate::gui::icon_system::StyledIcon,
    label: &str,
) -> egui::Response {
    eden_icon_button(
        ui,
        palette,
        icon,
        label,
        SETTINGS_ICON_BUTTON,
        body_size(palette) + 2.0,
    )
    .on_hover_text(label)
}

fn settings_dropdown(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    id: impl std::hash::Hash + std::fmt::Debug,
    width: f32,
    selected_text: impl Into<egui::WidgetText>,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    draw_dropdown_sized(
        ui,
        palette,
        id,
        width,
        Some(SETTINGS_CONTROL_HEIGHT),
        selected_text,
        add_contents,
    );
}

/// Label column of a row: wrapping text plus an optional info icon right after it.
/// Returns the text's response (click-sensitive).
fn setting_label(
    ui: &mut egui::Ui,
    text: &str,
    info: Option<&str>,
    palette: &ThemePalette,
) -> egui::Response {
    let icon_width = if info.is_some() {
        info_icon_size(palette) + 6.0
    } else {
        0.0
    };
    let max_width = (ui.available_width() - icon_width).max(40.0);
    let resp = ui
        .scope(|ui| {
            ui.set_max_width(max_width);
            ui.add(
                egui::Label::new(
                    RichText::new(text)
                        .size(body_size(palette))
                        .color(palette.text_normal),
                )
                .wrap()
                .sense(egui::Sense::click()),
            )
        })
        .inner;
    if let Some(hover_text) = info {
        ui.add_space(4.0);
        info_icon(ui, hover_text, palette);
    }
    resp
}

/// A checkbox row (no info icon); true when it changed.
fn setting_checkbox(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    checked: &mut bool,
    label: RichText,
    id: impl std::hash::Hash + std::fmt::Debug,
) -> bool {
    setting_toggle(ui, palette, checked, label.text(), None, id)
}

/// A checkbox row: label (with optional info icon) left, checkbox right. Clicking the label
/// toggles too. True when it changed.
fn setting_toggle(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    checked: &mut bool,
    label: &str,
    info: Option<&str>,
    id: impl std::hash::Hash + std::fmt::Debug,
) -> bool {
    let mut changed = false;
    let label_resp = setting_row(ui, palette, label, info, SETTINGS_CHECKBOX_HIT, |ui| {
        let resp = ui
            .allocate_ui_with_layout(
                egui::vec2(SETTINGS_CHECKBOX_HIT, SETTINGS_CHECKBOX_HIT),
                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                |ui| draw_checkbox_sized(ui, palette, checked, id, label, SETTINGS_CHECKBOX_SIZE),
            )
            .inner;
        changed |= resp.clicked();
    });
    if label_resp.is_some_and(|resp| resp.clicked()) {
        *checked = !*checked;
        changed = true;
    }
    changed
}

/// One settings row: label (wrapping, optional info icon) on the left, `control` right-aligned
/// in a column of `control_width`. The label column is whatever the control leaves, so a
/// long translated label wraps instead of running under the control. Returns the label's
/// response.
pub fn setting_row(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    label: &str,
    info: Option<&str>,
    control_width: f32,
    control: impl FnOnce(&mut egui::Ui),
) -> Option<egui::Response> {
    let mut label_resp = None;
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), SETTINGS_ROW_HEIGHT),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(control_width, SETTINGS_ROW_HEIGHT),
                egui::Layout::right_to_left(egui::Align::Center),
                control,
            );
            ui.add_space(SETTINGS_CONTROL_GAP);
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                label_resp = Some(setting_label(ui, label, info, palette));
            });
        },
    );
    ui.add_space(SETTINGS_ROW_GAP);
    label_resp
}

pub fn combo_box_string(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    id: impl std::hash::Hash + std::fmt::Debug,
    width: f32,
    value: &mut String,
    options: &[(&str, String)],
) -> bool {
    let mut changed = false;

    let selected_text = options
        .iter()
        .find(|(key, _)| *key == value)
        .map(|(_, label)| label.as_str())
        .unwrap_or("");

    settings_dropdown(ui, palette, id, width, selected_text, |ui| {
        for (key, label) in options {
            if ui.selectable_label(value == key, label).clicked() {
                *value = (*key).to_string();
                changed = true;
            }
        }
    });

    changed
}

/// Escape dismisses the settings window, except while a confirmation dialog is open: that
/// dialog consumes Escape itself as "cancel", so only the dialog closes.
fn escape_closes_settings(escape_pressed: bool, confirmation_open: bool) -> bool {
    escape_pressed && !confirmation_open
}

const SETTINGS_TABS: [(SettingsTab, &str); 4] = [
    (SettingsTab::General, "settings_tab_general"),
    (SettingsTab::Appearance, "settings_tab_appearance"),
    (SettingsTab::ViewDefaults, "settings_tab_view_defaults"),
    (SettingsTab::Hotkeys, "settings_tab_hotkeys"),
];

/// Bigger text, spacing and hit targets for the settings window only.
fn apply_settings_style(ui: &mut egui::Ui, palette: &ThemePalette) {
    let body = body_size(palette);
    let mut style = (**ui.style()).clone();
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::proportional(body + 3.0),
    );
    for text_style in [egui::TextStyle::Body, egui::TextStyle::Button] {
        style
            .text_styles
            .insert(text_style, egui::FontId::proportional(body));
    }
    style.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::proportional(body - 2.0),
    );
    style.spacing.item_spacing = egui::vec2(10.0, 6.0);
    style.spacing.interact_size = egui::vec2(40.0, SETTINGS_CONTROL_HEIGHT);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.override_text_valign = Some(egui::Align::Center);
    ui.set_style(style);
}

pub fn draw_settings_window(
    ctx: &egui::Context,
    settings: &mut SettingsWindow,
    i18n: &mut I18n,
    palette: &ThemePalette,
) -> Option<SettingsAction> {
    let mut action = None;

    if !settings.open {
        return None;
    }

    let mut should_close = false;

    // 🌑 Dark background overlay (modal effect); clicking it dismisses the window
    let modal_bg_clicked = egui::Area::new(egui::Id::new("settings_modal_bg"))
        .order(egui::Order::Middle)
        .interactable(true)
        .show(ctx, |ui| {
            let rect = ctx.content_rect();
            ui.painter()
                .rect_filled(rect, 0.0, palette.modal_background_effect_color);
            ui.interact(
                rect,
                ui.id().with("settings_modal_bg_click"),
                egui::Sense::click(),
            )
            .clicked()
        })
        .inner;

    if modal_bg_clicked {
        should_close = true;
    }

    let confirmation_open = settings.show_reset_favorites_confirmation
        || settings.show_reset_confirmation
        || settings.show_clear_directory_cache_confirmation;
    if escape_closes_settings(
        ctx.input(|i| i.key_pressed(egui::Key::Escape)),
        confirmation_open,
    ) {
        should_close = true;
    }

    let mut settings_open = settings.open;

    let window_size = ctx.content_rect().size();
    egui::Window::new(i18n.tr("settings"))
        .collapsible(false)
        .resizable(true)
        .default_size(clamp_dialog_size(SETTINGS_WINDOW_DEFAULT_SIZE, window_size))
        .min_size(clamp_dialog_size(SETTINGS_WINDOW_MIN_SIZE, window_size))
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .open(&mut settings_open)
        .frame(
            egui::Frame::popup(&ctx.style_of(ctx.theme()))
                .corner_radius(egui::CornerRadius::same(palette.large_radius)),
        )
        .show(ctx, |ui| {
            apply_settings_style(ui, palette);

            ui.horizontal(|ui| {
                for (tab, key) in SETTINGS_TABS {
                    if eden_toggle_button(ui, palette, settings.selected_tab == tab, &i18n.tr(key))
                        .clicked()
                    {
                        settings.selected_tab = tab;
                    }
                }
            });
            ui.separator();

            // SCROLLABLE CONTENT
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // Keep content clear of the scroll bar.
                    ui.set_max_width(ui.available_width() - 14.0);
                    action = match settings.selected_tab {
                        SettingsTab::Hotkeys => {
                            draw_hotkeys_tab(ui, palette, i18n);
                            None
                        }
                        SettingsTab::Appearance => draw_appearance_tab(ui, settings, palette, i18n),
                        SettingsTab::ViewDefaults => {
                            draw_view_defaults_tab(ui, settings, palette, i18n)
                        }
                        SettingsTab::General => draw_general_tab(ui, settings, palette, i18n),
                    };
                });

            // These confirmation dialogs live outside the selected-tab content so they
            // remain visible if the user changes tabs while a destructive action is pending.
            if settings.show_reset_favorites_confirmation {
                if let Some(confirmed) = draw_confirmation_dialog(
                    ctx,
                    palette,
                    i18n,
                    &i18n.tr("settings_favorites_reset_confirm"),
                    &[
                        &i18n.tr("settings_favorite_reset_confirm_label1"),
                        &i18n.tr("settings_favorite_reset_confirm_label2"),
                    ],
                ) {
                    settings.show_reset_favorites_confirmation = false;
                    if confirmed {
                        action = Some(SettingsAction::ResetFavourites);
                    }
                }
            }

            if settings.show_reset_confirmation {
                if let Some(confirmed) = draw_confirmation_dialog(
                    ctx,
                    palette,
                    i18n,
                    &i18n.tr("settings_reset_confirm"),
                    &[
                        &i18n.tr("settings_reset_confirm_label1"),
                        &i18n.tr("settings_reset_confirm_label2"),
                    ],
                ) {
                    settings.show_reset_confirmation = false;
                    if confirmed {
                        action = Some(SettingsAction::ResetToDefaults);
                    }
                }
            }

            if settings.show_clear_directory_cache_confirmation {
                if let Some(confirmed) = draw_confirmation_dialog(
                    ctx,
                    palette,
                    i18n,
                    &i18n.tr("settings_clear_directory_cache_confirm"),
                    &[
                        &i18n.tr("settings_clear_directory_cache_confirm_label1"),
                        &i18n.tr("settings_clear_directory_cache_confirm_label2"),
                    ],
                ) {
                    settings.show_clear_directory_cache_confirmation = false;
                    if confirmed {
                        action = Some(SettingsAction::ClearDirectoryCache);
                    }
                }
            }
        });
    if should_close || !settings_open {
        settings.open = false;
    }
    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_path_tail_keeps_short_strings() {
        assert_eq!(truncate_path_tail("C:/data", 40), "C:/data");
        let exact = "a".repeat(40);
        assert_eq!(truncate_path_tail(&exact, 40), exact);
    }

    #[test]
    fn truncate_path_tail_keeps_last_chars_of_long_ascii() {
        let s = format!("C:/{}/end", "x".repeat(60));
        let out = truncate_path_tail(&s, 40);
        assert_eq!(out, format!("...{}", &s[s.len() - 40..]));
        assert!(out.ends_with("/end"));
    }

    #[test]
    fn truncate_path_tail_does_not_split_multibyte_chars() {
        // Each Japanese char is 3 bytes, so a byte-based cut at len - 40 lands mid-char.
        let s = format!("C:/{}", "日本語のフォルダ".repeat(6));
        assert!(s.len() > 40);
        let out = truncate_path_tail(&s, 40);
        let chars: Vec<char> = s.chars().collect();
        let tail: String = chars[chars.len() - 40..].iter().collect();
        assert_eq!(out, format!("...{tail}"));
        assert_eq!(out.chars().count(), 43);
    }

    #[test]
    fn escape_closes_settings_only_without_an_open_confirmation() {
        assert!(escape_closes_settings(true, false));
        assert!(!escape_closes_settings(true, true));
        assert!(!escape_closes_settings(false, false));
        assert!(!escape_closes_settings(false, true));
    }

    #[test]
    fn hotkey_groups_list_every_binding_exactly_once() {
        let mut seen: Vec<usize> = HOTKEY_GROUPS
            .iter()
            .flat_map(|(_, indices)| indices.iter().copied())
            .collect();
        seen.sort_unstable();

        assert_eq!(seen, (0..HOTKEYS.len()).collect::<Vec<_>>());
    }
}
