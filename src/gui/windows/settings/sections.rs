//! The non-theme sections of the settings window. Each draws flat rows and
//! returns the action to run, if any.

use super::*;

/// Files: what the list shows. Advanced: default widths and the folder cache.
pub(super) fn draw_files_section(
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

    settings_section(ui, palette, None, |ui| {
        changed |= setting_toggle(
            ui,
            palette,
            &mut settings.current_settings.show_hidden_files_folders,
            &i18n.tr("settings_show_hidden_files_folders"),
            Some(&i18n.tr("tooltip_settings_show_hidden_files_folders")),
            "settings_show_hidden_files_folders",
        );
        changed |= setting_toggle(
            ui,
            palette,
            &mut settings.current_settings.folder_scanning_enabled,
            &i18n.tr("settings_directoryscanning"),
            Some(&i18n.tr("tooltip_settings_directoryscanning")),
            "settings_directoryscanning",
        );
        changed |= setting_toggle(
            ui,
            palette,
            &mut settings.current_settings.show_item_viewer_icons,
            &i18n.tr("settings_show_item_viewer_file_icons"),
            None,
            "settings_show_item_viewer_file_icons",
        );
        changed |= setting_toggle(
            ui,
            palette,
            &mut settings.current_settings.extra_actions_bar,
            &i18n.tr("settings_enable_extra_action_buttons_in_item_viewer"),
            None,
            "settings_enable_extra_action_buttons_in_item_viewer",
        );
    });

    settings_section(
        ui,
        palette,
        Some(&i18n.tr("settings_view_defaults_columns")),
        |ui| {
            for column in DEFAULT_FILE_COLUMNS {
                let mut visible = current_order.contains(&column);
                if setting_toggle(
                    ui,
                    palette,
                    &mut visible,
                    &file_column_label(i18n, column),
                    None,
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
        },
    );

    settings_advanced(ui, palette, i18n, "files", |ui| {
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
            let label = format!(
                "{}: {}",
                i18n.tr("settings_default_column_width"),
                file_column_label(i18n, column)
            );
            if drag_row(ui, palette, &label, None, &mut width, 20.0..=1000.0, 1.0) {
                settings.current_settings.item_viewer_file_column_sizes[index] = width;
                changed = true;
            }
        }

        if button_row(
            ui,
            palette,
            &i18n.tr("settings_clear_directory_cache"),
            None,
            &i18n.tr("settings_action_clear"),
        ) {
            settings.show_clear_directory_cache_confirmation = true;
        }
    });

    changed.then_some(SettingsAction::ApplySettings)
}

/// Interface: layout of the toolbar, status bar, sidebar and item list.
pub(super) fn draw_interface_section(
    ui: &mut egui::Ui,
    settings: &mut SettingsWindow,
    palette: &ThemePalette,
    i18n: &I18n,
) -> Option<SettingsAction> {
    let mut changed = false;
    let prefs = &mut settings.current_settings.ui;

    settings_section(ui, palette, None, |ui| {
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
        changed |= pref_toggle(
            ui,
            palette,
            i18n,
            "settings_status_bar_visible",
            &mut prefs.status_bar.visible,
        );
        if button_row(
            ui,
            palette,
            &i18n.tr("settings_toolbar_explorer_like"),
            None,
            &i18n.tr("settings_action_apply"),
        ) {
            prefs.toolbar = ToolbarPrefs::explorer_like();
            prefs.status_bar = StatusBarPrefs::explorer_like();
            prefs.sidebar = SidebarPrefs::explorer_like();
            changed = true;
        }
    });

    settings_advanced(ui, palette, i18n, "interface", |ui| {
        settings_heading(ui, palette, &i18n.tr("settings_toolbar"));
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
            changed |= pref_toggle(ui, palette, i18n, key, value);
        }

        settings_heading(ui, palette, &i18n.tr("settings_status_bar"));
        let s = &mut prefs.status_bar;
        for (key, value) in [
            ("settings_status_bar_item_count", &mut s.item_count),
            ("settings_status_bar_selection_size", &mut s.selection_size),
            ("settings_status_bar_view_toggles", &mut s.view_toggles),
            ("settings_status_bar_counters", &mut s.counters),
        ] {
            changed |= pref_toggle(ui, palette, i18n, key, value);
        }

        settings_heading(ui, palette, &i18n.tr("settings_sidebar"));
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
            changed |= pref_toggle(ui, palette, i18n, key, value);
        }
    });

    changed.then_some(SettingsAction::ApplySettings)
}

/// A toggle row bound to one preference; true when it changed.
fn pref_toggle(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    i18n: &I18n,
    key: &str,
    value: &mut bool,
) -> bool {
    setting_toggle(ui, palette, value, &i18n.tr(key), None, ("pref", key))
}

/// Integrations with the rest of Windows and shell tools.
pub(super) fn draw_integrations_section(
    ui: &mut egui::Ui,
    settings: &mut SettingsWindow,
    palette: &ThemePalette,
    i18n: &I18n,
) -> Option<SettingsAction> {
    let mut changed = false;
    settings_section(ui, palette, None, |ui| {
        changed |= setting_toggle(
            ui,
            palette,
            &mut settings.current_settings.windows_context_menu_enabled,
            &i18n.tr("settings_contextmenu_enable"),
            Some(&i18n.tr("tooltip_settings_contextmenu")),
            "settings_contextmenu_enable",
        );
        changed |= setting_toggle(
            ui,
            palette,
            &mut settings.current_settings.zoxide_auto_add,
            &i18n.tr("settings_zoxide_auto_add"),
            Some(&i18n.tr("tooltip_settings_zoxide_auto_add")),
            "settings_zoxide_auto_add",
        );
        let mut percent = (settings.current_settings.zoxide_popup_opacity * 100.0).round();
        if slider_row(
            ui,
            palette,
            &i18n.tr("settings_zoxide_popup_opacity"),
            Some(&i18n.tr("tooltip_settings_zoxide_popup_opacity")),
            &mut percent,
            50.0..=100.0,
            "%",
            1.0,
        ) {
            settings.current_settings.zoxide_popup_opacity = percent / 100.0;
            changed = true;
        }
    });
    changed.then_some(SettingsAction::ApplySettings)
}

pub(super) fn draw_general_section(
    ui: &mut egui::Ui,
    settings: &mut SettingsWindow,
    palette: &ThemePalette,
    i18n: &mut I18n,
) -> Option<SettingsAction> {
    let mut action = None;

    settings_section(ui, palette, None, |ui| {
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

        // Starting folder: the current value is the description.
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
        setting_row(
            ui,
            palette,
            &i18n.tr("settings_startpath"),
            Some(&path_text),
            2.0 * icon_button_size(palette) + 8.0,
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

        // Window size
        let mut window_size_changed = false;
        setting_row(
            ui,
            palette,
            &i18n.tr("settings_windowsize"),
            None,
            SETTINGS_COMBO_WIDTH + icon_button_size(palette) + 8.0,
            |ui| {
                let is_fullscreen = matches!(
                    settings.current_settings.window_size_mode,
                    WindowSizeMode::FullScreen
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
            },
        );

        if let WindowSizeMode::Custom { width, height } =
            &mut settings.current_settings.window_size_mode
        {
            window_size_changed |= drag_row(
                ui,
                palette,
                &i18n.tr("settings_windowsize_width"),
                None,
                width,
                800.0..=4000.0,
                1.0,
            );
            window_size_changed |= drag_row(
                ui,
                palette,
                &i18n.tr("settings_windowsize_height"),
                None,
                height,
                600.0..=3000.0,
                1.0,
            );
        }
        if window_size_changed {
            action = Some(SettingsAction::ApplySettings);
        }

        // Date and time
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

    settings_advanced(ui, palette, i18n, "general", |ui| {
        if button_row(
            ui,
            palette,
            &i18n.tr("settings_favorites_reset"),
            Some(&i18n.tr("tooltip_settings_favorites_reset")),
            &i18n.tr("settings_action_reset"),
        ) {
            settings.show_reset_favorites_confirmation = true;
        }
        if button_row(
            ui,
            palette,
            &i18n.tr("settings_reset"),
            None,
            &i18n.tr("settings_action_reset"),
        ) {
            settings.show_reset_confirmation = true;
        }
    });

    action
}
