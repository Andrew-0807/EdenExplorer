//! Appearance section: base theme, accent, font, backdrop, and under Advanced
//! the sizes, shapes and every color grouped by the part of the UI it paints.
//! It edits the palette of the chosen mode (light or dark, active or not) in the `ThemeCustomizer`; the existing
//! handler applies, saves, exports and imports.

use super::*;
use crate::core::utils::fonts::get_font_list;
use crate::gui::backdrop::{Backdrop, needs_restart, transparent_surface};
use crate::gui::folder_icons::IconStyle;
use crate::gui::selection::{HoverStyle, SelectionStyle};
use crate::gui::theme::{ThemeMode, apply_font_to_context, regenerate_base_derived_colors};
use crate::gui::theme_presets::{Density, ThemePreset, apply_density, preset_palette};
use crate::gui::windows::enums::ThemeCustomizerAction;
use crate::gui::windows::structs::ThemeCustomizer;
use egui::Color32;

/// Which base theme the dropdown shows.
#[derive(Clone, Copy, PartialEq)]
enum Base {
    Light,
    Dark,
    System,
}

pub(super) fn draw_appearance_section(
    ui: &mut egui::Ui,
    settings: &mut SettingsWindow,
    customizer: &mut ThemeCustomizer,
    mode: ThemeMode,
    palette: &ThemePalette,
    i18n: &I18n,
) -> Option<SettingsAction> {
    let ctx = ui.ctx().clone();
    let mut prefs_changed = false;
    let mut theme_changed = false;

    // Which mode the controls below edit; it need not be the active one.
    let mut edit = customizer.edit_mode.unwrap_or(mode);
    settings_section(ui, palette, None, |ui| {
        choice_row(
            ui,
            palette,
            i18n,
            "settings_theme_edit_mode",
            Some("settings_theme_edit_mode_desc"),
            &mut edit,
            &[
                (ThemeMode::Light, "theme_light"),
                (ThemeMode::Dark, "theme_dark"),
            ],
        );
    });
    customizer.edit_mode = Some(edit);
    let prefs = &mut settings.current_settings.ui;
    let p = match edit {
        ThemeMode::Dark => &mut customizer.dark_palette,
        ThemeMode::Light => &mut customizer.light_palette,
    };

    settings_section(ui, palette, None, |ui| {
        // Base theme
        let mut base = if prefs.theme_follow_system {
            Base::System
        } else if mode == ThemeMode::Dark {
            Base::Dark
        } else {
            Base::Light
        };
        let before = base;
        choice_row(
            ui,
            palette,
            i18n,
            "settings_theme_base",
            Some("settings_theme_base_desc"),
            &mut base,
            &[
                (Base::Light, "theme_light"),
                (Base::Dark, "theme_dark"),
                (Base::System, "settings_theme_system"),
            ],
        );
        if base != before {
            match base {
                Base::System => {
                    prefs.theme_follow_system = true;
                    crate::core::system_theme::mark_system_theme_dirty();
                }
                Base::Light | Base::Dark => {
                    prefs.theme_follow_system = false;
                    customizer.requested_mode = Some(if base == Base::Dark {
                        ThemeMode::Dark
                    } else {
                        ThemeMode::Light
                    });
                }
            }
            prefs_changed = true;
        }

        // Preset
        setting_row(
            ui,
            palette,
            &i18n.tr("settings_theme_preset"),
            Some(&i18n.tr("settings_theme_preset_desc")),
            SETTINGS_COMBO_WIDTH,
            |ui| {
                settings_dropdown(
                    ui,
                    palette,
                    "theme_preset_picker",
                    SETTINGS_COMBO_WIDTH,
                    i18n.tr("theme_presets"),
                    |ui| {
                        for preset in ThemePreset::ALL {
                            if ui
                                .selectable_label(false, i18n.tr(preset.label_key()))
                                .clicked()
                            {
                                let updated = preset_palette(preset, edit, p);
                                *p = updated;
                                theme_changed = true;
                                ui.close();
                            }
                        }
                    },
                );
            },
        );

        // Accent
        if color_row(
            ui,
            palette,
            &i18n.tr("settings_theme_accent"),
            Some(&i18n.tr("settings_theme_accent_desc")),
            &mut p.primary,
        ) {
            p.follow_system_accent = false;
            regenerate_base_derived_colors(p, edit == ThemeMode::Dark);
            theme_changed = true;
        }
        if setting_toggle(
            ui,
            palette,
            &mut p.follow_system_accent,
            &i18n.tr("theme_follow_accent"),
            None,
            "theme_follow_accent",
        ) {
            if p.follow_system_accent {
                // Picked up on the next frame, like a Windows accent change.
                crate::core::system_theme::mark_system_theme_dirty();
            }
            theme_changed = true;
        }

        // Font and size
        setting_row(
            ui,
            palette,
            &i18n.tr("theme_font"),
            None,
            SETTINGS_COMBO_WIDTH,
            |ui| {
                if let Some(font) = font_selector(ui, palette, "theme_font_selector", &p.font_name)
                {
                    p.font_name = font;
                    if edit == mode {
                        apply_font_to_context(&ctx, p);
                    }
                    theme_changed = true;
                }
            },
        );
        theme_changed |= slider_row(
            ui,
            palette,
            &i18n.tr("theme_font_size"),
            Some(&i18n.tr("theme_font_size_desc")),
            &mut p.text_size,
            8.0..=24.0,
            "",
            0.5,
        );
        let mut percent = (prefs.zoom * 100.0).round();
        if slider_row(
            ui,
            palette,
            &i18n.tr("settings_zoom"),
            None,
            &mut percent,
            50.0..=300.0,
            "%",
            5.0,
        ) {
            prefs.zoom = percent / 100.0;
            prefs_changed = true;
        }

        // Backdrop
        let backdrop_note = needs_restart(p.backdrop, transparent_surface())
            .then(|| i18n.tr("theme_backdrop_restart"));
        let options: Vec<(Backdrop, &str)> =
            Backdrop::ALL.iter().map(|b| (*b, b.label_key())).collect();
        theme_changed |= choice_row_desc(
            ui,
            palette,
            i18n,
            "theme_backdrop",
            backdrop_note.as_deref(),
            &mut p.backdrop,
            &options,
        );
        theme_changed |= percent_slider_row(
            ui,
            palette,
            &i18n.tr("theme_backdrop_tint_opacity"),
            &mut p.backdrop_tint_opacity,
        );
    });

    settings_advanced(ui, palette, i18n, "appearance", |ui| {
        // Density
        setting_row(
            ui,
            palette,
            &i18n.tr("theme_density"),
            None,
            3.0 * 92.0,
            |ui| {
                for density in Density::ALL.into_iter().rev() {
                    if settings_button(ui, palette, &i18n.tr(density.label_key()))
                        .is_some_and(|r| r.clicked())
                    {
                        apply_density(p, density);
                        theme_changed = true;
                    }
                }
            },
        );
        for (key, value, range, speed) in [
            (
                "theme_tooltip_textsize",
                &mut p.tooltip_text_size,
                8.0..=24.0,
                0.2,
            ),
            (
                "theme_explorer_rowheight",
                &mut p.row_height,
                8.0..=40.0,
                0.5,
            ),
            (
                "theme_sidebar_iconsize",
                &mut p.sidebar_icon_size,
                8.0..=32.0,
                0.2,
            ),
            ("theme_tab_iconsize", &mut p.tab_icon_size, 8.0..=32.0, 0.2),
            (
                "theme_sidebar_item_spacing_y",
                &mut p.sidebar_item_spacing_y,
                -0.3..=2.0,
                0.1,
            ),
            (
                "theme_selection_stroke_width",
                &mut p.selection_stroke_width,
                0.5..=4.0,
                0.05,
            ),
        ] {
            theme_changed |= drag_row(ui, palette, &i18n.tr(key), None, value, range, speed);
        }
        for (key, value, max) in [
            ("theme_selection_radius", &mut p.selection_radius, 12u8),
            ("theme_radius_small", &mut p.small_radius, 16),
            ("theme_radius_medium", &mut p.medium_radius, 16),
            ("theme_radius_large", &mut p.large_radius, 24),
        ] {
            theme_changed |= byte_row(ui, palette, &i18n.tr(key), value, max);
        }
        let selection: Vec<(SelectionStyle, &str)> = SelectionStyle::ALL
            .iter()
            .map(|s| (*s, s.label_key()))
            .collect();
        theme_changed |= choice_row(
            ui,
            palette,
            i18n,
            "theme_selection_style",
            None,
            &mut p.selection_style,
            &selection,
        );
        let hovers: Vec<(HoverStyle, &str)> = HoverStyle::ALL
            .iter()
            .map(|s| (*s, s.label_key()))
            .collect();
        theme_changed |= choice_row(
            ui,
            palette,
            i18n,
            "theme_hover_style",
            None,
            &mut p.hover_style,
            &hovers,
        );
        let icons: Vec<(IconStyle, &str)> =
            IconStyle::ALL.iter().map(|s| (*s, s.label_key())).collect();
        theme_changed |= choice_row(
            ui,
            palette,
            i18n,
            "theme_icon_style",
            None,
            &mut p.icon_style,
            &icons,
        );
        theme_changed |= setting_toggle(
            ui,
            palette,
            &mut p.show_separators,
            &i18n.tr("theme_show_separators"),
            None,
            "theme_show_separators",
        );
        theme_changed |= setting_toggle(
            ui,
            palette,
            &mut p.bg_gradient,
            &i18n.tr("theme_bg_gradient"),
            None,
            "theme_bg_gradient",
        );

        settings_collapsible(ui, palette, i18n, "settings_theme_customize_colors", |ui| {
            theme_changed |= draw_color_groups(ui, palette, i18n, p);
        });

        // Theme file
        setting_row(
            ui,
            palette,
            &i18n.tr("settings_theme_file"),
            customizer.status.map(|(key, _)| i18n.tr(key)).as_deref(),
            3.0 * 92.0,
            |ui| {
                for (key, action) in [
                    ("theme_reset", ThemeCustomizerAction::ResetToDefaults(edit)),
                    ("theme_import", ThemeCustomizerAction::ImportTheme(edit)),
                    ("theme_export", ThemeCustomizerAction::ExportTheme(edit)),
                ] {
                    if settings_button(ui, palette, &i18n.tr(key)).is_some_and(|r| r.clicked()) {
                        customizer.pending_action = Some(action);
                    }
                }
            },
        );
    });

    if theme_changed && customizer.pending_action.is_none() {
        customizer.pending_action = Some(ThemeCustomizerAction::ThemeUpdated(edit));
    }
    if customizer.pending_action.is_some() {
        ctx.request_repaint();
    }
    prefs_changed.then_some(SettingsAction::ApplySettings)
}

/// Every color token, grouped by the part of the UI it paints.
fn draw_color_groups(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    i18n: &I18n,
    p: &mut ThemePalette,
) -> bool {
    let mut changed = false;
    let mut group = |ui: &mut egui::Ui, title_key: &str, colors: Vec<(&str, &mut Color32)>| {
        settings_section(ui, palette, Some(&i18n.tr(title_key)), |ui| {
            for (key, color) in colors {
                changed |= color_row(ui, palette, &i18n.tr(key), None, color);
            }
        });
    };

    group(
        ui,
        "settings_theme_area_window",
        vec![
            (
                "theme_colors_application_background",
                &mut p.application_bg_color,
            ),
            ("theme_colors_content_bg", &mut p.content_bg),
            ("theme_colors_borders_default", &mut p.borders_default),
            ("theme_colors_borders_active", &mut p.borders_active),
            (
                "theme_colors_modal_overlay",
                &mut p.modal_background_effect_color,
            ),
            ("theme_colors_bg_gradient_end", &mut p.bg_gradient_end),
        ],
    );
    group(
        ui,
        "settings_theme_area_sidebar",
        vec![
            ("theme_colors_sidebar_bg", &mut p.sidebar_bg),
            ("theme_colors_favorite", &mut p.button_favorite_fill),
            ("theme_colors_drive_normal", &mut p.drive_usage_normal),
            ("theme_colors_drive_warning", &mut p.drive_usage_warning),
            ("theme_colors_drive_critical", &mut p.drive_usage_critical),
            (
                "theme_colors_drive_background",
                &mut p.drive_usage_background,
            ),
            ("theme_colors_drive_text", &mut p.drive_usage_text),
        ],
    );
    group(
        ui,
        "settings_theme_area_items",
        vec![
            ("theme_colors_row_bg", &mut p.row_bg),
            ("theme_colors_row_selected_bg", &mut p.row_selected_bg),
            ("theme_colors_selection_stroke", &mut p.selection_stroke),
            ("theme_colors_folder", &mut p.folder_color),
            (
                "theme_colors_folder_desktop",
                &mut p.known_folder_colors.desktop,
            ),
            (
                "theme_colors_folder_documents",
                &mut p.known_folder_colors.documents,
            ),
            (
                "theme_colors_folder_downloads",
                &mut p.known_folder_colors.downloads,
            ),
            (
                "theme_colors_folder_pictures",
                &mut p.known_folder_colors.pictures,
            ),
            (
                "theme_colors_folder_music",
                &mut p.known_folder_colors.music,
            ),
            (
                "theme_colors_folder_videos",
                &mut p.known_folder_colors.videos,
            ),
            ("theme_colors_icon", &mut p.icon_color),
            ("theme_colors_icon_hover", &mut p.icon_colored_hover),
            ("theme_colors_icon_windows", &mut p.icon_windows),
            ("theme_colors_checkbox_bg", &mut p.checkbox_bg_default),
            ("theme_colors_checkbox_hover", &mut p.checkbox_bg_hover),
            ("theme_colors_checkbox_active", &mut p.checkbox_bg_active),
            (
                "theme_colors_checkbox_check",
                &mut p.checkbox_checkmark_color,
            ),
        ],
    );
    group(
        ui,
        "settings_theme_area_tabs",
        vec![
            ("theme_colors_tab_close_normal", &mut p.tab_close_normal),
            ("theme_colors_tab_close_hover", &mut p.tab_close_hover),
            ("theme_colors_tab_add_hover", &mut p.tab_add_hover),
            ("theme_colors_pinned_tab", &mut p.pinned_tab_color),
            ("theme_colors_tab_text_selected", &mut p.tab_text_selected),
            (
                "theme_colors_separator_handle",
                &mut p.button_seperator_handle_fill,
            ),
        ],
    );
    group(
        ui,
        "settings_theme_area_menus",
        vec![
            ("theme_colors_widget_bg", &mut p.widget_bg),
            ("theme_colors_faint_bg", &mut p.faint_bg),
            ("theme_colors_widget_text", &mut p.widget_text),
            ("theme_colors_widget_text_hover", &mut p.widget_text_hover),
            ("theme_colors_tooltip_text", &mut p.tooltip_text_color),
            ("theme_colors_primary_hover", &mut p.primary_hover),
            ("theme_colors_primary_active", &mut p.primary_active),
        ],
    );
    group(
        ui,
        "settings_theme_area_text",
        vec![
            ("theme_colors_text_normal", &mut p.text_normal),
            ("theme_colors_text_header", &mut p.text_header_section),
            (
                "theme_colors_text_selected",
                &mut p.item_viewer_row_text_selected,
            ),
        ],
    );
    changed
}

fn font_selector(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    id: &str,
    current_font: &str,
) -> Option<String> {
    let fonts = get_font_list();
    let mut picked = None;
    settings_dropdown(
        ui,
        palette,
        egui::Id::new(id),
        SETTINGS_COMBO_WIDTH,
        current_font.to_string(),
        |ui| {
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    for font in fonts.iter() {
                        if ui
                            .selectable_label(font == current_font, font.as_str())
                            .clicked()
                        {
                            picked = Some(font.clone());
                            ui.close();
                        }
                    }
                });
        },
    );
    picked
}
