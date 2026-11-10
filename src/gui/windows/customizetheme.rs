use crate::core::i18n::I18n;
use crate::core::utils::fonts::get_font_list;
use crate::gui::backdrop::{Backdrop, needs_restart, transparent_surface};
use crate::gui::theme::{
    ThemeMode, ThemePalette, apply_font_to_context, error_color, regenerate_base_derived_colors,
};
use crate::gui::theme_presets::{ThemePreset, preset_palette};
use crate::gui::utils::rgba_color_edit_button;
use crate::gui::utils::widgets::{
    apply_eden_visual_overrides, draw_dropdown, eden_button, eden_text_label, eden_toggle_button,
};
use crate::gui::windows::enums::ThemeCustomizerAction;
use crate::gui::windows::structs::ThemeCustomizer;
use crate::gui::windows::windowsoverrides::clamp_dialog_size;
use eframe::egui;
use egui::Color32;

fn selectable_mode(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    current: ThemeMode,
    target: ThemeMode,
    label: &str,
) -> bool {
    eden_toggle_button(ui, palette, current == target, label).clicked()
}

/// Draws rows of a two-column grid (label | control) and remembers whether
/// any control changed.
pub(crate) struct EditorRows<'a> {
    pub i18n: &'a I18n,
    /// Palette of the active theme, used to style the editor itself.
    pub style: &'a ThemePalette,
    pub changed: bool,
}

impl EditorRows<'_> {
    fn label(&self, ui: &mut egui::Ui, key: &str) {
        eden_text_label(ui, self.style, &self.i18n.tr(key));
    }

    fn control<R>(&self, ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
        let style = self.style;
        let inner = ui
            .with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                apply_eden_visual_overrides(ui, style);
                add(ui)
            })
            .inner;
        ui.end_row();
        inner
    }

    pub fn color(&mut self, ui: &mut egui::Ui, key: &str, color: &mut Color32) -> bool {
        self.label(ui, key);
        let changed = self.control(ui, |ui| rgba_color_edit_button(ui, color).changed());
        self.changed |= changed;
        changed
    }

    pub fn number(
        &mut self,
        ui: &mut egui::Ui,
        key: &str,
        value: &mut f32,
        range: std::ops::RangeInclusive<f32>,
        speed: f64,
    ) -> bool {
        self.label(ui, key);
        let changed = self.control(ui, |ui| {
            ui.add_sized(
                egui::vec2(90.0, 0.0),
                egui::DragValue::new(value).range(range).speed(speed),
            )
            .changed()
        });
        self.changed |= changed;
        changed
    }

    pub fn byte(
        &mut self,
        ui: &mut egui::Ui,
        key: &str,
        value: &mut u8,
        range: std::ops::RangeInclusive<u8>,
    ) -> bool {
        self.label(ui, key);
        let changed = self.control(ui, |ui| {
            ui.add_sized(
                egui::vec2(90.0, 0.0),
                egui::DragValue::new(value).range(range).speed(0.1),
            )
            .changed()
        });
        self.changed |= changed;
        changed
    }

    pub fn toggle(&mut self, ui: &mut egui::Ui, key: &str, value: &mut bool) -> bool {
        self.label(ui, key);
        let style = self.style;
        let changed = self.control(ui, |ui| {
            crate::gui::utils::widgets::draw_checkbox(ui, style, value, ("theme_toggle", key), key)
                .clicked()
        });
        self.changed |= changed;
        changed
    }

    /// A drop-down over `options` (value, i18n key).
    pub fn choice<T: Copy + PartialEq>(
        &mut self,
        ui: &mut egui::Ui,
        key: &str,
        value: &mut T,
        options: &[(T, &str)],
    ) -> bool {
        self.label(ui, key);
        let i18n = self.i18n;
        let style = self.style;
        let current = options
            .iter()
            .find(|(v, _)| v == value)
            .map(|(_, k)| i18n.tr(k))
            .unwrap_or_default();
        let changed = self.control(ui, |ui| {
            let mut changed = false;
            draw_dropdown(ui, style, ("theme_choice", key), 180.0, current, |ui| {
                for (option, label_key) in options {
                    if ui
                        .selectable_label(option == value, i18n.tr(label_key))
                        .clicked()
                    {
                        *value = *option;
                        changed = true;
                        ui.close();
                    }
                }
            });
            changed
        });
        self.changed |= changed;
        changed
    }
}

/// One collapsible group of rows.
fn section(
    ui: &mut egui::Ui,
    rows: &mut EditorRows,
    title_key: &str,
    open: bool,
    add: impl FnOnce(&mut egui::Ui, &mut EditorRows),
) {
    let title = egui::RichText::new(rows.i18n.tr(title_key))
        .size(rows.style.text_size + 1.0)
        .color(rows.style.text_header_section)
        .strong();
    egui::CollapsingHeader::new(title)
        .id_salt(("theme_section", title_key))
        .default_open(open)
        .show(ui, |ui| {
            egui::Grid::new(("theme_grid", title_key))
                .num_columns(2)
                .spacing([12.0, 6.0])
                .min_col_width(200.0)
                .show(ui, |ui| add(ui, rows));
        });
}

pub fn draw_theme_customizer(
    i18n: &I18n,
    ctx: &egui::Context,
    customizer: &mut ThemeCustomizer,
    palette: &ThemePalette,
) -> Option<ThemeCustomizerAction> {
    let mut action = None;

    if !customizer.open {
        return None;
    }

    // 🌑 Dark background overlay (modal effect); clicking it dismisses the window
    let modal_bg_clicked = egui::Area::new(egui::Id::new("theme_modal_bg"))
        .order(egui::Order::Middle)
        .interactable(true)
        .show(ctx, |ui| {
            let rect = ctx.content_rect();
            ui.painter()
                .rect_filled(rect, 0.0, palette.modal_background_effect_color);
            ui.interact(
                rect,
                ui.id().with("theme_modal_bg_click"),
                egui::Sense::click(),
            )
            .clicked()
        })
        .inner;

    if modal_bg_clicked || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        customizer.open = false;
    }

    let mut open = customizer.open;
    egui::Window::new(i18n.tr("theme_title"))
        .collapsible(false)
        .resizable(false)
        .fixed_size(clamp_dialog_size([620.0, 600.0], ctx.content_rect().size()))
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .open(&mut open)
        .frame(
            egui::Frame::popup(&ctx.style_of(ctx.theme()))
                .corner_radius(egui::CornerRadius::same(palette.large_radius)),
        )
        .show(ctx, |ui| {
            ui.set_width(ui.available_width());
            // TOP SECTION: select which palette to edit
            ui.horizontal(|ui| {
                if selectable_mode(
                    ui,
                    palette,
                    customizer.selected_mode,
                    ThemeMode::Dark,
                    &i18n.tr("theme_dark"),
                ) {
                    customizer.selected_mode = ThemeMode::Dark;
                }

                if selectable_mode(
                    ui,
                    palette,
                    customizer.selected_mode,
                    ThemeMode::Light,
                    &i18n.tr("theme_light"),
                ) {
                    customizer.selected_mode = ThemeMode::Light;
                }

                ui.separator();
                if let Some(preset) = preset_picker(ui, i18n, palette) {
                    let mode = customizer.selected_mode;
                    let edited = customizer.palette_mut(mode);
                    *edited = preset_palette(preset, mode, edited);
                    action = Some(ThemeCustomizerAction::ThemeUpdated(mode));
                }
            });
            ui.add_space(4.0);

            let mode = customizer.selected_mode;
            let editing_palette = customizer.palette_mut(mode);
            let mut rows = EditorRows {
                i18n,
                style: palette,
                changed: false,
            };

            // SCROLLABLE CONTENT
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(ui.available_height() - 40.0)
                .show(ui, |ui| {
                    apply_eden_visual_overrides(ui, palette);
                    draw_sections(ui, ctx, &mut rows, editing_palette, mode);
                });

            ui.separator();

            // FOOTER
            ui.horizontal(|ui| {
                if eden_button(ui, palette, &i18n.tr("theme_export")).clicked() {
                    action = Some(ThemeCustomizerAction::ExportTheme(mode));
                }

                if eden_button(ui, palette, &i18n.tr("theme_import")).clicked() {
                    action = Some(ThemeCustomizerAction::ImportTheme(mode));
                }
                if eden_button(ui, palette, &i18n.tr("theme_reset")).clicked() {
                    action = Some(ThemeCustomizerAction::ResetToDefaults(mode));
                }
                if let Some((key, is_error)) = customizer.status {
                    let color = if is_error {
                        error_color(ui.visuals().dark_mode)
                    } else {
                        palette.text_normal
                    };
                    ui.label(egui::RichText::new(i18n.tr(key)).color(color));
                }
            });

            if rows.changed && action.is_none() {
                action = Some(ThemeCustomizerAction::ThemeUpdated(mode));
            }
        });
    customizer.open &= open;

    action
}

fn preset_picker(ui: &mut egui::Ui, i18n: &I18n, palette: &ThemePalette) -> Option<ThemePreset> {
    let mut picked = None;
    draw_dropdown(
        ui,
        palette,
        "theme_preset_picker",
        170.0,
        i18n.tr("theme_presets"),
        |ui| {
            for preset in ThemePreset::ALL {
                if ui
                    .selectable_label(false, i18n.tr(preset.label_key()))
                    .clicked()
                {
                    picked = Some(preset);
                    ui.close();
                }
            }
        },
    );
    picked
}

fn draw_sections(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    rows: &mut EditorRows,
    p: &mut ThemePalette,
    mode: ThemeMode,
) {
    section(ui, rows, "theme_typography", true, |ui, rows| {
        rows.number(ui, "theme_textsize", &mut p.text_size, 8.0..=24.0, 0.2);
        rows.number(
            ui,
            "theme_tooltip_textsize",
            &mut p.tooltip_text_size,
            8.0..=24.0,
            0.2,
        );
        rows.number(
            ui,
            "theme_explorer_rowheight",
            &mut p.row_height,
            8.0..=40.0,
            0.5,
        );
        rows.number(
            ui,
            "theme_sidebar_iconsize",
            &mut p.sidebar_icon_size,
            8.0..=32.0,
            0.2,
        );
        rows.number(
            ui,
            "theme_tab_iconsize",
            &mut p.tab_icon_size,
            8.0..=32.0,
            0.2,
        );
        rows.number(
            ui,
            "theme_sidebar_item_spacing_y",
            &mut p.sidebar_item_spacing_y,
            -0.3..=2.0,
            0.1,
        );

        rows.label(ui, "theme_font");
        let style = rows.style;
        let new_font = rows.control(ui, |ui| {
            font_selector(ui, style, "theme_font_selector", &p.font_name)
        });
        if let Some(font) = new_font {
            p.font_name = font;
            apply_font_to_context(ctx, p);
            rows.changed = true;
        }
    });

    section(ui, rows, "theme_core_colors", true, |ui, rows| {
        if rows.color(ui, "theme_colors_primary", &mut p.primary) {
            regenerate_base_derived_colors(p, mode == ThemeMode::Dark);
        }
        rows.color(ui, "theme_colors_primary_hover", &mut p.primary_hover);
        rows.color(ui, "theme_colors_primary_active", &mut p.primary_active);
        rows.color(ui, "theme_colors_borders_default", &mut p.borders_default);
        rows.color(ui, "theme_colors_borders_active", &mut p.borders_active);
        rows.color(
            ui,
            "theme_colors_application_background",
            &mut p.application_bg_color,
        );
    });

    section(ui, rows, "theme_section_window", true, |ui, rows| {
        let options: Vec<(Backdrop, &str)> =
            Backdrop::ALL.iter().map(|b| (*b, b.label_key())).collect();
        rows.choice(ui, "theme_backdrop", &mut p.backdrop, &options);
        rows.number(
            ui,
            "theme_backdrop_tint_opacity",
            &mut p.backdrop_tint_opacity,
            0.0..=1.0,
            0.01,
        );
        rows.color(ui, "theme_colors_sidebar_bg", &mut p.sidebar_bg);
        rows.color(ui, "theme_colors_content_bg", &mut p.content_bg);
        rows.toggle(ui, "theme_show_separators", &mut p.show_separators);
        rows.toggle(ui, "theme_bg_gradient", &mut p.bg_gradient);
        rows.color(ui, "theme_colors_bg_gradient_end", &mut p.bg_gradient_end);
    });
    if needs_restart(p.backdrop, transparent_surface()) {
        ui.label(
            egui::RichText::new(rows.i18n.tr("theme_backdrop_restart"))
                .color(rows.style.text_normal)
                .italics(),
        );
    }

    section(ui, rows, "theme_section_text", false, |ui, rows| {
        rows.color(ui, "theme_colors_text_normal", &mut p.text_normal);
        rows.color(ui, "theme_colors_text_header", &mut p.text_header_section);
        rows.color(
            ui,
            "theme_colors_text_selected",
            &mut p.item_viewer_row_text_selected,
        );
        rows.color(ui, "theme_colors_tooltip_text", &mut p.tooltip_text_color);
        rows.color(
            ui,
            "theme_colors_tab_text_selected",
            &mut p.tab_text_selected,
        );
        rows.color(ui, "theme_colors_widget_text", &mut p.widget_text);
        rows.color(
            ui,
            "theme_colors_widget_text_hover",
            &mut p.widget_text_hover,
        );
    });

    section(ui, rows, "theme_section_surfaces", false, |ui, rows| {
        rows.color(ui, "theme_colors_widget_bg", &mut p.widget_bg);
        rows.color(ui, "theme_colors_faint_bg", &mut p.faint_bg);
        rows.color(ui, "theme_colors_row_bg", &mut p.row_bg);
        rows.color(ui, "theme_colors_row_selected_bg", &mut p.row_selected_bg);
        rows.color(
            ui,
            "theme_colors_modal_overlay",
            &mut p.modal_background_effect_color,
        );
    });

    section(ui, rows, "theme_section_icons", false, |ui, rows| {
        rows.color(ui, "theme_colors_icon", &mut p.icon_color);
        rows.color(ui, "theme_colors_icon_hover", &mut p.icon_colored_hover);
        rows.color(ui, "theme_colors_icon_windows", &mut p.icon_windows);
    });

    section(ui, rows, "theme_section_tabs", false, |ui, rows| {
        rows.color(ui, "theme_colors_tab_close_normal", &mut p.tab_close_normal);
        rows.color(ui, "theme_colors_tab_close_hover", &mut p.tab_close_hover);
        rows.color(ui, "theme_colors_tab_add_hover", &mut p.tab_add_hover);
        rows.color(ui, "theme_colors_pinned_tab", &mut p.pinned_tab_color);
        rows.color(ui, "theme_colors_favorite", &mut p.button_favorite_fill);
        rows.color(
            ui,
            "theme_colors_separator_handle",
            &mut p.button_seperator_handle_fill,
        );
    });

    section(ui, rows, "theme_section_drives", false, |ui, rows| {
        rows.color(ui, "theme_colors_drive_normal", &mut p.drive_usage_normal);
        rows.color(ui, "theme_colors_drive_warning", &mut p.drive_usage_warning);
        rows.color(
            ui,
            "theme_colors_drive_critical",
            &mut p.drive_usage_critical,
        );
        rows.color(
            ui,
            "theme_colors_drive_background",
            &mut p.drive_usage_background,
        );
        rows.color(ui, "theme_colors_drive_text", &mut p.drive_usage_text);
    });

    section(ui, rows, "theme_section_checkbox", false, |ui, rows| {
        rows.color(ui, "theme_colors_checkbox_bg", &mut p.checkbox_bg_default);
        rows.color(ui, "theme_colors_checkbox_hover", &mut p.checkbox_bg_hover);
        rows.color(
            ui,
            "theme_colors_checkbox_active",
            &mut p.checkbox_bg_active,
        );
        rows.color(
            ui,
            "theme_colors_checkbox_check",
            &mut p.checkbox_checkmark_color,
        );
    });

    section(ui, rows, "theme_section_shape", false, |ui, rows| {
        rows.byte(ui, "theme_radius_small", &mut p.small_radius, 0..=16);
        rows.byte(ui, "theme_radius_medium", &mut p.medium_radius, 0..=16);
        rows.byte(ui, "theme_radius_large", &mut p.large_radius, 0..=24);
    });
}

fn font_selector(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    label: &str,
    current_font: &str,
) -> Option<String> {
    let fonts = get_font_list();

    let mut selected_text = current_font.to_string();
    let mut changed = false;

    draw_dropdown(
        ui,
        palette,
        egui::Id::new(label),
        200.0,
        selected_text.clone(),
        |ui| {
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    for font in fonts.iter() {
                        let rich_text = egui::RichText::new(font.as_str());

                        if ui
                            .selectable_label(font == current_font, rich_text)
                            .clicked()
                        {
                            selected_text = font.clone();
                            changed = true;
                            ui.close();
                        }
                    }
                });
        },
    );

    if changed { Some(selected_text) } else { None }
}
