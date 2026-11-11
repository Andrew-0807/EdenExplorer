use crate::core::fs::{MY_PC_PATH, MY_RECYCLE_BIN_PATH};
use crate::core::herdr;
use crate::core::i18n::I18n;
use crate::core::icons::IconCache;
use crate::core::portable;
use crate::gui::customization::{
    ToolbarItem, ToolbarLayout, ToolbarPrefs, ToolbarState, toolbar_item_enabled, toolbar_items,
};
use crate::gui::icon_system::{AppIcon, StyledIcon, draw_icon, paint_icon};
use crate::gui::theme::{ThemePalette, error_color};
use crate::gui::utils::{SortColumn, SortKey};
use crate::gui::utils::{clickable_icon, expand_environment_variables};
use crate::gui::windows::containers::enums::{ItemViewerContextAction, ItemViewerNavAction};
use crate::gui::windows::containers::structs::{
    Breadcrumb, ItemViewerDisplayMode, ItemViewerNavBarAction, RenderedBreadcrumb, TabView,
};
use eframe::egui;
use egui::FontId;
use egui::text::{CCursor, CCursorRange};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn draw_itemviewer_navigation_bar(
    ui: &mut egui::Ui,
    i18n: &I18n,
    icon_cache: &IconCache,
    tab: &mut TabView,
    tab_id: u64,
    palette: &ThemePalette,
    is_favorited: bool,
    drag_active: bool,
    drag_hover_target: Option<PathBuf>,
    extra_actions_bar: bool,
    toolbar_prefs: ToolbarPrefs,
    clipboard_has_files: bool,
) -> ItemViewerNavBarAction {
    let mut action = ItemViewerNavBarAction::default();
    let tabbar_rect = ui.available_rect_before_wrap();
    ui.set_clip_rect(tabbar_rect);

    let pointer_pos = ui.input(|i| i.pointer.interact_pos().or_else(|| i.pointer.hover_pos()));
    let pointer_released =
        ui.input(|i| i.pointer.any_released() && i.pointer.interact_pos().is_some());
    let hovered_target_ref = drag_hover_target.as_ref();
    let mut breadcrumb_drop_target: Option<PathBuf> = None;
    let pointer_in_tabbar = pointer_pos
        .map(|pos| tabbar_rect.contains(pos))
        .unwrap_or(false);
    let can_go_back = tab.nav.can_go_back();
    let can_go_forward = tab.nav.can_go_forward();
    let is_recycle_bin = tab.nav.is_recycle_bin();

    ui.horizontal(|ui| {
        ui.add_space(1.5);
        let toolbar_action = {
            let toolbar = toolbar_context(
                i18n,
                palette,
                tab,
                toolbar_prefs,
                extra_actions_bar,
                is_favorited,
                clipboard_has_files,
            );
            let inline = (toolbar_prefs.layout == ToolbarLayout::SingleRow).then_some(&toolbar);
            draw_navigation_bar_buttons(
                ui,
                i18n,
                palette,
                tab.nav.is_root(),
                is_recycle_bin,
                can_go_back,
                can_go_forward,
                inline,
            )
        };

        merge_toolbar_action(&mut action, toolbar_action);
        if action.toggle_gallery {
            tab.display_mode = match tab.display_mode {
                ItemViewerDisplayMode::Details => ItemViewerDisplayMode::Gallery,
                ItemViewerDisplayMode::Gallery => ItemViewerDisplayMode::Details,
            };
        }

        navbar_separator(ui, palette);

        let breadcrumb_width = ui.available_width();

        if tab.breadcrumb_path_editing {
            let text_edit_id = ui.id().with(("breadcrumbs_path_edit", tab_id));

            if tab.breadcrumb_select_all_on_focus {
                let mut state =
                    egui::widgets::text_edit::TextEditState::load(ui.ctx(), text_edit_id)
                        .unwrap_or_default();
                let cursor_end = CCursor::new(tab.breadcrumb_path_buffer.chars().count());
                state
                    .cursor
                    .set_char_range(Some(CCursorRange::two(CCursor::new(0), cursor_end)));
                state.store(ui.ctx(), text_edit_id);
            }

            let mut offset_x = 0.0;
            if tab.breadcrumb_path_error {
                let t = (ui.input(|i| i.time) - tab.breadcrumb_path_error_animation_time) as f32;

                if t < 0.4 {
                    let frequency = 30.0_f32;
                    let amplitude = 4.0 * (1.0 - t / 0.4); // decay
                    offset_x = (t * frequency).sin() * amplitude;
                }
            }

            ui.add_space(offset_x);

            let time_since_error = ui.input(|i| i.time) - tab.breadcrumb_path_error_animation_time;
            let error_strength = (1.0 - time_since_error * 2.0).clamp(0.0, 1.0);

            let stroke_color = if tab.breadcrumb_path_error {
                let error = error_color(ui.visuals().dark_mode);
                let alpha = (255.0 * error_strength) as u8;
                egui::Color32::from_rgba_unmultiplied(error.r(), error.g(), error.b(), alpha)
            } else {
                ui.visuals().widgets.inactive.bg_stroke.color
            };

            let frame = egui::Frame::NONE
                .stroke(egui::Stroke::new(1.5, stroke_color))
                .corner_radius(egui::CornerRadius::same(palette.medium_radius));

            let mut resp = frame
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut tab.breadcrumb_path_buffer)
                            .id(text_edit_id)
                            .frame(if tab.breadcrumb_path_error {
                                egui::Frame::NONE
                            } else {
                                egui::Frame::default()
                            })
                            .desired_width(ui.available_width() - 40.0)
                            .font(FontId::new(
                                palette.text_size,
                                egui::FontFamily::Proportional,
                            )),
                    )
                })
                .inner;

            if resp.changed() {
                tab.breadcrumb_path_error = false;
            }

            if tab.breadcrumb_path_error && resp.hovered() {
                resp = resp.on_hover_text(
                    egui::RichText::new(i18n.tr("tooltip_path_does_not_exist"))
                        .size(palette.tooltip_text_size)
                        .color(palette.tooltip_text_color),
                );
            }

            if !tab.breadcrumb_just_started_editing || tab.breadcrumb_path_error {
                resp.request_focus();
                tab.breadcrumb_just_started_editing = true;
            }

            if resp.has_focus() {
                tab.breadcrumb_select_all_on_focus = false;
            }

            action.is_breadcrumb_path_edit_active = resp.has_focus();

            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));

            let mut exit_edit_mode = false;

            if enter {
                let input = tab.breadcrumb_path_buffer.trim().trim_matches('"');
                let expanded_input = expand_environment_variables(input);
                let new_path = PathBuf::from(&expanded_input);

                if new_path.exists() {
                    action.nav_to = Some(new_path);
                    exit_edit_mode = true;
                } else {
                    println!(
                        "{}: {} ({}: {})",
                        i18n.tr("tooltip_invalid_path"),
                        tab.breadcrumb_path_buffer,
                        i18n.tr("tooltip_invalid_path_expanded"),
                        expanded_input
                    );
                    tab.breadcrumb_path_error = true;
                    tab.breadcrumb_path_error_animation_time = ui.input(|i| i.time);
                }
            } else if escape || resp.lost_focus() {
                tab.breadcrumb_path_buffer = tab.nav.current.to_string_lossy().to_string();
                exit_edit_mode = true;
            }

            if exit_edit_mode {
                tab.breadcrumb_path_editing = false;
                tab.breadcrumb_just_started_editing = false;
                tab.breadcrumb_path_error = false;
                tab.breadcrumb_path_error_animation_time = 0.0;

                ui.memory_mut(|mem| mem.surrender_focus(text_edit_id));
            }
        } else if tab.nav.is_root() || tab.nav.is_recycle_bin() {
            let pc_icon_path = PathBuf::from("C:\\");
            if tab.nav.is_recycle_bin() {
                draw_icon(
                    ui,
                    AppIcon::Trash.regular(),
                    16.0,
                    palette.text_header_section,
                    &i18n.tr("recycle_bin"),
                );
            } else if let Some(icon) = icon_cache.get(&pc_icon_path, true) {
                ui.add(egui::Image::new(&icon).fit_to_exact_size(egui::vec2(16.0, 16.0)));
            }

            if ui
                .add(
                    egui::Label::new(
                        egui::RichText::new(if tab.nav.is_recycle_bin() {
                            i18n.tr("recycle_bin")
                        } else {
                            i18n.tr("thispc")
                        })
                        .size(palette.text_size)
                        .color(palette.text_header_section),
                    )
                    .selectable(false)
                    .sense(egui::Sense::click()),
                )
                .clicked()
            {
                action.nav_to = Some(if tab.nav.is_recycle_bin() {
                    PathBuf::from(MY_RECYCLE_BIN_PATH)
                } else {
                    PathBuf::from(MY_PC_PATH)
                });
            }
        } else {
            let font_id = egui::FontId::new(palette.text_size, egui::FontFamily::Proportional);
            let breadcrumbs = build_breadcrumbs(&tab.nav.current);
            let segments = layout_breadcrumbs(ui, &breadcrumbs, breadcrumb_width, &font_id);
            let mut first = true;
            let mut breadcrumbs_right = 0.0;

            for crumb in &segments {
                if !first {
                    let old_spacing = ui.spacing().item_spacing;
                    ui.spacing_mut().item_spacing.x = 10.0;

                    ui.label(
                        egui::RichText::new(">")
                            .size(palette.text_size)
                            .color(palette.text_header_section),
                    );

                    ui.spacing_mut().item_spacing = old_spacing;
                }
                first = false;

                let resp = draw_breadcrumb(ui, crumb, palette);

                handle_breadcrumb_drag(
                    ui,
                    &resp,
                    crumb,
                    &segments,
                    palette,
                    drag_active,
                    hovered_target_ref,
                    pointer_pos,
                    pointer_released,
                    &mut breadcrumb_drop_target,
                    &mut action,
                );

                handle_breadcrumb_hover(ui, &resp);

                handle_breadcrumb_click(&resp, crumb, &segments, drag_active, &mut action);

                breadcrumbs_right = resp.rect.right();
            }

            // ---- Detect click in empty area to enter path editing ----
            let available_rect = ui.available_rect_before_wrap();
            let empty_area = egui::Rect::from_min_max(
                egui::pos2(breadcrumbs_right, available_rect.top()),
                available_rect.right_bottom(),
            );

            if ui
                .interact(
                    empty_area,
                    ui.id().with("breadcrumb_empty"),
                    egui::Sense::click(),
                )
                .clicked()
            {
                tab.breadcrumb_path_editing = true;
                tab.breadcrumb_path_buffer = tab.nav.current.to_string_lossy().to_string();
            }
        }
    });

    if action.nav.is_none() && pointer_in_tabbar && !tab.breadcrumb_path_editing {
        if can_go_back && ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Extra1)) {
            action.nav = Some(ItemViewerNavAction::Back);
        } else if can_go_forward
            && ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Extra2))
        {
            action.nav = Some(ItemViewerNavAction::Forward);
        }
    }

    action.move_files_to_breadcrumb_dir = breadcrumb_drop_target;
    action
}

fn nav_icon_button(
    ui: &mut egui::Ui,
    icon: StyledIcon,
    palette: &ThemePalette,
    enabled: bool,
    hover_text: &str,
) -> egui::Response {
    let size = palette.text_size + 2.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
    paint_icon(
        ui,
        rect,
        icon,
        size,
        if enabled {
            ui.visuals().text_color()
        } else {
            ui.visuals().weak_text_color()
        },
    );

    if enabled && resp.hovered() {
        paint_icon(ui, resp.rect, icon, size, palette.primary);
    }

    let resp = resp.on_hover_text(
        egui::RichText::new(hover_text)
            .size(palette.tooltip_text_size)
            .color(palette.tooltip_text_color),
    );

    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_navigation_bar_buttons(
    ui: &mut egui::Ui,
    i18n: &I18n,
    palette: &ThemePalette,
    is_root: bool,
    is_recycle_bin: bool,
    can_go_back: bool,
    can_go_forward: bool,
    inline_toolbar: Option<&ToolbarContext>,
) -> ItemViewerNavBarAction {
    let mut action = ItemViewerNavBarAction::default();

    // Navigation buttons
    if nav_icon_button(
        ui,
        AppIcon::ArrowLeft.regular(),
        palette,
        can_go_back,
        &i18n.tr("tooltip_nav_back"),
    )
    .clicked()
        && can_go_back
    {
        action.nav = Some(ItemViewerNavAction::Back);
    }

    if nav_icon_button(
        ui,
        AppIcon::ArrowRight.regular(),
        palette,
        can_go_forward,
        &i18n.tr("tooltip_nav_forward"),
    )
    .clicked()
        && can_go_forward
    {
        action.nav = Some(ItemViewerNavAction::Forward);
    }

    let can_go_up = !is_root && !is_recycle_bin;
    if nav_icon_button(
        ui,
        AppIcon::ArrowUp.regular(),
        palette,
        can_go_up,
        &i18n.tr("tooltip_nav_up"),
    )
    .clicked()
        && can_go_up
    {
        action.nav = Some(ItemViewerNavAction::Up);
    }

    if clickable_icon(
        ui,
        AppIcon::ArrowsClockwise.regular(),
        palette,
        &i18n.tr("tooltip_refresh"),
    )
    .on_hover_text(
        egui::RichText::new(i18n.tr("tooltip_refresh"))
            .size(palette.tooltip_text_size)
            .color(palette.tooltip_text_color),
    )
    .on_hover_cursor(egui::CursorIcon::PointingHand)
    .clicked()
    {
        action.refresh_current_directory = true;
    }

    if let Some(toolbar) = inline_toolbar {
        let items = toolbar_items(&toolbar.prefs, toolbar.extra_actions_bar);
        if !items.is_empty() {
            ui.add_space(2.0);
            merge_toolbar_action(&mut action, draw_toolbar_items(ui, toolbar, &items));
        }
    }

    action
}

/// Everything the toolbar buttons need to know about the current view.
pub struct ToolbarContext<'a> {
    pub i18n: &'a I18n,
    pub palette: &'a ThemePalette,
    pub prefs: ToolbarPrefs,
    pub extra_actions_bar: bool,
    pub is_favorited: bool,
    pub current_dir: &'a Path,
    pub display_mode: ItemViewerDisplayMode,
    pub selected_paths: &'a std::collections::HashSet<PathBuf>,
    pub state: ToolbarState,
    pub primary_sort: Option<SortKey>,
}

/// Builds the toolbar context for `tab`.
pub fn toolbar_context<'a>(
    i18n: &'a I18n,
    palette: &'a ThemePalette,
    tab: &'a TabView,
    prefs: ToolbarPrefs,
    extra_actions_bar: bool,
    is_favorited: bool,
    clipboard_has_files: bool,
) -> ToolbarContext<'a> {
    ToolbarContext {
        i18n,
        palette,
        prefs,
        extra_actions_bar,
        is_favorited,
        current_dir: &tab.nav.current,
        display_mode: tab.display_mode,
        selected_paths: &tab.explorer_state.selected_paths,
        state: ToolbarState {
            is_root: tab.nav.is_root(),
            is_recycle_bin: tab.nav.is_recycle_bin(),
            has_selection: !tab.explorer_state.selected_paths.is_empty(),
            clipboard_has_files,
        },
        primary_sort: tab.sort_keys.first().copied(),
    }
}

/// Explorer-style command bar: its own row under the address bar.
pub fn draw_command_bar(ui: &mut egui::Ui, toolbar: &ToolbarContext) -> ItemViewerNavBarAction {
    let items = toolbar_items(&toolbar.prefs, toolbar.extra_actions_bar);
    ui.horizontal_centered(|ui| {
        ui.add_space(4.0);
        draw_toolbar_items(ui, toolbar, &items)
    })
    .inner
}

/// One toolbar button: icon, optional label, hover fill. Disabled buttons
/// are dimmed and ignore clicks.
fn toolbar_button(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    icon: StyledIcon,
    label: Option<&str>,
    enabled: bool,
    tooltip: &str,
    icon_color: Option<egui::Color32>,
) -> egui::Response {
    let icon_size = palette.text_size + 2.0;
    let font_id = FontId::proportional(palette.text_size);
    let text_color = if enabled {
        ui.visuals().text_color()
    } else {
        ui.visuals().weak_text_color()
    };
    let galley = label.map(|text| {
        ui.painter()
            .layout_no_wrap(text.to_owned(), font_id.clone(), text_color)
    });
    let pad = if galley.is_some() { 6.0 } else { 3.0 };
    let width = pad * 2.0 + icon_size + galley.as_ref().map_or(0.0, |g| g.size().x + 5.0);
    let height = icon_size + 8.0;
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, height), sense);

    if enabled && resp.hovered() {
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same(palette.medium_radius),
            ui.visuals().widgets.hovered.weak_bg_fill,
        );
    }
    let icon_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left() + pad, rect.center().y - icon_size * 0.5),
        egui::Vec2::splat(icon_size),
    );
    let color = if !enabled {
        ui.visuals().weak_text_color()
    } else {
        icon_color.unwrap_or(text_color)
    };
    paint_icon(ui, icon_rect, icon, icon_size, color);
    if let Some(galley) = galley {
        let pos = egui::pos2(
            icon_rect.right() + 5.0,
            rect.center().y - galley.size().y * 0.5,
        );
        ui.painter().galley(pos, galley, text_color);
    }

    let resp = resp.on_hover_text(
        egui::RichText::new(tooltip)
            .size(palette.tooltip_text_size)
            .color(palette.tooltip_text_color),
    );
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

fn draw_toolbar_items(
    ui: &mut egui::Ui,
    t: &ToolbarContext,
    items: &[ToolbarItem],
) -> ItemViewerNavBarAction {
    let mut action = ItemViewerNavBarAction::default();
    let i18n = t.i18n;
    let palette = t.palette;
    let selected = || {
        let mut paths: Vec<PathBuf> = t.selected_paths.iter().cloned().collect();
        paths.sort();
        paths
    };
    let old_spacing = ui.spacing().item_spacing.x;
    ui.spacing_mut().item_spacing.x = 2.0;

    for &item in items {
        if item == ToolbarItem::Separator {
            ui.add_space(2.0);
            navbar_separator(ui, palette);
            ui.add_space(2.0);
            continue;
        }
        let enabled = toolbar_item_enabled(item, t.state);
        let label_text = i18n.tr(item.label_key());
        let label = item
            .shows_label(t.prefs.labels)
            .then_some(label_text.as_str());
        let (icon, tooltip, color) = match item {
            ToolbarItem::NewFolder => (
                AppIcon::FolderPlus.regular(),
                i18n.tr("tooltip_newfolder"),
                None,
            ),
            ToolbarItem::NewFile => (
                AppIcon::FilePlus.regular(),
                i18n.tr("tooltip_newfile"),
                None,
            ),
            ToolbarItem::Cut => (AppIcon::ActionCut.regular(), label_text.clone(), None),
            ToolbarItem::Copy => (AppIcon::ActionCopy.regular(), label_text.clone(), None),
            ToolbarItem::Paste => (AppIcon::ActionPaste.regular(), label_text.clone(), None),
            ToolbarItem::Rename => (AppIcon::ActionRename.regular(), label_text.clone(), None),
            ToolbarItem::Delete => (AppIcon::ActionDelete.regular(), label_text.clone(), None),
            ToolbarItem::Properties => (
                AppIcon::ActionProperties.regular(),
                label_text.clone(),
                None,
            ),
            ToolbarItem::Sort => (AppIcon::Sliders.regular(), label_text.clone(), None),
            ToolbarItem::View => (
                AppIcon::ImagesSquare.regular(),
                i18n.tr("tooltip_gallery_view"),
                (t.display_mode == ItemViewerDisplayMode::Gallery).then_some(palette.primary),
            ),
            ToolbarItem::Favorite => {
                let (icon, color) = if t.is_favorited {
                    (AppIcon::Star.filled(), palette.button_favorite_fill)
                } else {
                    (AppIcon::Star.regular(), palette.tooltip_text_color)
                };
                let tooltip = if !enabled {
                    i18n.tr("tooltip_favorites_disabled")
                } else if t.is_favorited {
                    i18n.tr("tooltip_favorites_remove")
                } else {
                    i18n.tr("tooltip_favorites_add")
                };
                (icon, tooltip, Some(color))
            }
            ToolbarItem::Terminal => (
                AppIcon::Terminal.regular(),
                i18n.tr("tooltip_open_terminal"),
                None,
            ),
            ToolbarItem::Separator => continue,
        };

        let resp = toolbar_button(ui, palette, icon, label, enabled, &tooltip, color);
        match item {
            ToolbarItem::Sort => {
                egui::Popup::menu(&resp).show(|ui| {
                    for (column, key) in [
                        (SortColumn::Name, "explorer_cols_name"),
                        (SortColumn::Modified, "explorer_cols_modified"),
                        (SortColumn::Type, "explorer_cols_type"),
                        (SortColumn::Size, "explorer_cols_size"),
                        (SortColumn::Created, "explorer_cols_created"),
                        (SortColumn::Tags, "explorer_cols_tags"),
                    ] {
                        let current = t.primary_sort.filter(|k| k.column == column);
                        let arrow = match current {
                            Some(k) if k.ascending => "  ▲",
                            Some(_) => "  ▼",
                            None => "",
                        };
                        if ui
                            .selectable_label(current.is_some(), format!("{}{arrow}", i18n.tr(key)))
                            .clicked()
                        {
                            action.sort_by = Some(column);
                            ui.close();
                        }
                    }
                });
            }
            ToolbarItem::View if t.prefs.labels => {
                egui::Popup::menu(&resp).show(|ui| {
                    for (mode, key) in [
                        (ItemViewerDisplayMode::Details, "toolbar_view_details"),
                        (ItemViewerDisplayMode::Gallery, "toolbar_view_gallery"),
                    ] {
                        if ui
                            .selectable_label(t.display_mode == mode, i18n.tr(key))
                            .clicked()
                        {
                            action.toggle_gallery = t.display_mode != mode;
                            ui.close();
                        }
                    }
                });
            }
            ToolbarItem::Terminal => {
                if resp.clicked() && enabled {
                    open_default_terminal(t.current_dir);
                }
                if enabled && !herdr::is_missing() {
                    if resp.secondary_clicked() {
                        herdr::refresh_in_background(ui.ctx());
                    }
                    herdr_popup(&resp, i18n, t.current_dir, false);
                }
            }
            _ if resp.clicked() && enabled => match item {
                ToolbarItem::NewFolder => action.create_folder = true,
                ToolbarItem::NewFile => action.create_file = true,
                ToolbarItem::Cut => {
                    action.context_action = Some(ItemViewerContextAction::Cut(selected()))
                }
                ToolbarItem::Copy => {
                    action.context_action = Some(ItemViewerContextAction::Copy(selected()))
                }
                ToolbarItem::Paste => action.context_action = Some(ItemViewerContextAction::Paste),
                ToolbarItem::Rename => action.rename_path = selected().into_iter().next(),
                ToolbarItem::Delete => {
                    action.context_action = Some(ItemViewerContextAction::Delete {
                        paths: selected(),
                        permanent: t.state.is_recycle_bin,
                    })
                }
                ToolbarItem::Properties => {
                    action.context_action = Some(ItemViewerContextAction::Properties(selected()))
                }
                ToolbarItem::View => action.toggle_gallery = true,
                ToolbarItem::Favorite => {
                    if t.is_favorited {
                        action.remove_favorite = true;
                    } else {
                        action.add_favorite = true;
                    }
                }
                ToolbarItem::Sort | ToolbarItem::Terminal | ToolbarItem::Separator => {}
            },
            _ => {}
        }
    }

    ui.spacing_mut().item_spacing.x = old_spacing;
    action
}

/// A separator line, or just the same gap when the theme hides separators.
fn navbar_separator(ui: &mut egui::Ui, palette: &ThemePalette) {
    if palette.show_separators {
        ui.separator();
    } else {
        ui.add_space(ui.spacing().item_spacing.x + 2.0);
    }
}

fn terminal_start_dir(current_dir: &Path) -> PathBuf {
    if current_dir.to_string_lossy() == MY_PC_PATH || !current_dir.exists() {
        dirs::home_dir().unwrap_or_else(|| current_dir.to_path_buf())
    } else {
        current_dir.to_path_buf()
    }
}

/// Right-click menu of the terminal button, anchored below the button so it
/// never covers it. `force_open` keeps it open (used by render scenes).
pub(crate) fn herdr_popup(
    resp: &egui::Response,
    i18n: &I18n,
    current_dir: &Path,
    force_open: bool,
) {
    let open_cmd = if force_open || resp.secondary_clicked() {
        Some(egui::SetOpenCommand::Bool(true))
    } else if resp.clicked() {
        Some(egui::SetOpenCommand::Bool(false))
    } else {
        None
    };
    egui::Popup::menu(resp)
        .open_memory(open_cmd)
        .show(|ui| herdr_menu(ui, i18n, current_dir));
}

/// Workspace, pane and tab actions of the herdr menu.
fn herdr_menu(ui: &mut egui::Ui, i18n: &I18n, current_dir: &Path) {
    let dir = terminal_start_dir(current_dir);
    let ctx = ui.ctx().clone();
    match herdr::snapshot() {
        herdr::Snapshot::Unknown => {
            ui.add_enabled(false, egui::Button::new(i18n.tr("herdr_loading")));
        }
        herdr::Snapshot::NoServer => {
            ui.add_enabled(false, egui::Button::new(i18n.tr("herdr_new_workspace")))
                .on_disabled_hover_text(i18n.tr("herdr_no_server"));
        }
        herdr::Snapshot::Workspaces(workspaces) => {
            if ui.button(i18n.tr("herdr_new_workspace")).clicked() {
                herdr::create_workspace(&dir, &ctx);
                ui.close();
            }
            for ws in &workspaces {
                ui.separator();
                let pane_label = format!("{} {}", i18n.tr("herdr_new_pane_in"), ws.label);
                let tab_label = format!("{} {}", i18n.tr("herdr_new_tab_in"), ws.label);
                let pane = ui.add_enabled(ws.anchor_pane.is_some(), egui::Button::new(pane_label));
                if pane.clicked()
                    && let Some(anchor) = &ws.anchor_pane
                {
                    herdr::create_pane(anchor, &dir, &ctx);
                    ui.close();
                }
                if ui.button(tab_label).clicked() {
                    herdr::create_tab(&ws.id, &dir, &ctx);
                    ui.close();
                }
            }
        }
    }
}

pub(crate) fn open_default_terminal(current_dir: &Path) {
    let start_dir = terminal_start_dir(current_dir);

    let launched = Command::new("wt.exe")
        .arg("-d")
        .arg(&start_dir)
        .spawn()
        .is_ok()
        || Command::new("powershell.exe")
            .current_dir(&start_dir)
            .spawn()
            .is_ok()
        || Command::new("cmd.exe")
            .current_dir(&start_dir)
            .spawn()
            .is_ok();

    if !launched {
        eprintln!("Failed to open terminal");
    }
}

fn measure_breadcrumb_width(ui: &egui::Ui, font_id: &egui::FontId, text: &str) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font_id.clone(), ui.visuals().text_color())
        .size()
        .x
}

fn truncate_breadcrumb_to_width(
    ui: &egui::Ui,
    font_id: &egui::FontId,
    text: &str,
    max_width: f32,
) -> (String, bool) {
    if measure_breadcrumb_width(ui, font_id, text) <= max_width {
        return (text.to_owned(), false);
    }

    let ellipsis = "...";
    let ellipsis_width = measure_breadcrumb_width(ui, font_id, ellipsis);

    let mut result = String::new();

    for ch in text.chars() {
        let candidate = format!("{result}{ch}");

        if measure_breadcrumb_width(ui, font_id, &candidate) + ellipsis_width > max_width {
            break;
        }

        result.push(ch);
    }

    result.push_str(ellipsis);

    (result, true)
}

fn build_breadcrumbs(path: &Path) -> Vec<Breadcrumb> {
    use std::path::Component;

    if portable::is_portable_path(path) {
        return portable::build_breadcrumb_segments(path)
            .unwrap_or_default()
            .into_iter()
            .map(|(label, path)| Breadcrumb { label, path })
            .collect();
    }

    let mut breadcrumbs = Vec::new();
    let mut current = PathBuf::new();
    let mut drive_prefix = None;

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => {
                let prefix_text = prefix.as_os_str().to_string_lossy().into_owned();
                current.push(prefix.as_os_str());
                drive_prefix = Some(prefix_text);

                breadcrumbs.push(Breadcrumb {
                    label: prefix.as_os_str().to_string_lossy().into_owned(),
                    path: current.clone(),
                });
            }

            Component::RootDir => {
                // A drive prefix by itself (for example `C:`) is drive-relative
                // on Windows. Build the root explicitly as `C:\\` so clicking
                // the drive breadcrumb cannot resolve against the process cwd.
                if let Some(prefix) = &drive_prefix {
                    current = PathBuf::from(format!("{prefix}\\"));
                } else {
                    current.push(Path::new("\\"));
                }
            }

            Component::Normal(name) => {
                current.push(name);

                breadcrumbs.push(Breadcrumb {
                    label: name.to_string_lossy().into_owned(),
                    path: current.clone(),
                });
            }

            _ => {}
        }
    }

    breadcrumbs
}

fn layout_breadcrumbs(
    ui: &egui::Ui,
    breadcrumbs: &[Breadcrumb],
    available_width: f32,
    font_id: &egui::FontId,
) -> Vec<RenderedBreadcrumb> {
    const SEPARATOR_WIDTH: f32 = 18.0;
    const ITEM_PADDING: f32 = 12.0;

    if breadcrumbs.is_empty() {
        return Vec::new();
    }

    // Measure every breadcrumb once.
    let measured: Vec<RenderedBreadcrumb> = breadcrumbs
        .iter()
        .map(|crumb| RenderedBreadcrumb {
            label: crumb.label.clone(),
            full_label: crumb.label.clone(),
            path: crumb.path.clone(),
            truncated: false,
            is_ellipsis: false,
            width: measure_breadcrumb_width(ui, font_id, &crumb.label) + ITEM_PADDING,
        })
        .collect();

    // Fast path: everything fits.
    let total_width: f32 = measured.iter().map(|c| c.width).sum::<f32>()
        + SEPARATOR_WIDTH * measured.len().saturating_sub(1) as f32;

    if total_width <= available_width {
        return measured;
    }

    let ellipsis_width = measure_breadcrumb_width(ui, font_id, "...") + ITEM_PADDING;

    let mut result = Vec::new();

    // Always keep the first breadcrumb.
    result.push(measured[0].clone());

    let mut used_width = measured[0].width;

    // Keep as many breadcrumbs from the right as possible.
    let mut right_side = Vec::new();

    for crumb in measured.iter().skip(1).rev() {
        let needed = crumb.width + SEPARATOR_WIDTH;

        // Leave room for:
        //   - separator before "..."
        //   - "..."
        //   - separator after "..."
        let reserved = SEPARATOR_WIDTH + ellipsis_width + SEPARATOR_WIDTH;

        if used_width + reserved + needed <= available_width {
            used_width += needed;
            right_side.push(crumb.clone());
        } else {
            break;
        }
    }

    right_side.reverse();

    let omitted = right_side.len() < measured.len() - 1;

    if omitted {
        result.push(RenderedBreadcrumb {
            label: "...".into(),
            full_label: "...".into(),
            path: measured[0].path.clone(),
            truncated: false,
            is_ellipsis: true,
            width: ellipsis_width,
        });
    }

    result.extend(right_side);

    // Truncate only the final breadcrumb if needed.
    if let Some(last_index) = result.len().checked_sub(1) {
        let consumed: f32 = result[..last_index]
            .iter()
            .map(|c| c.width + SEPARATOR_WIDTH)
            .sum();

        let remaining = (available_width - consumed).max(80.0);

        let full = result[last_index].full_label.clone();

        let (label, truncated) = truncate_breadcrumb_to_width(ui, font_id, &full, remaining);

        result[last_index].label = label;
        result[last_index].truncated = truncated;
    }

    result
}

fn draw_breadcrumb(
    ui: &mut egui::Ui,
    crumb: &RenderedBreadcrumb,
    palette: &ThemePalette,
) -> egui::Response {
    let inner = egui::Frame::NONE
        .fill(egui::Color32::TRANSPARENT)
        .inner_margin(egui::Margin::symmetric(6, 2))
        .corner_radius(egui::CornerRadius::same(palette.medium_radius))
        .show(ui, |ui| {
            let resp = ui.add(
                egui::Label::new(
                    egui::RichText::new(&crumb.label)
                        .size(palette.text_size)
                        .color(palette.text_header_section),
                )
                .selectable(false)
                .sense(egui::Sense::click()),
            );

            if crumb.truncated {
                resp.on_hover_text(&crumb.full_label)
            } else {
                resp
            }
        });

    inner.response.union(inner.inner)
}

fn handle_breadcrumb_hover(ui: &egui::Ui, resp: &egui::Response) {
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
}

fn handle_breadcrumb_click(
    resp: &egui::Response,
    crumb: &RenderedBreadcrumb,
    segments: &[RenderedBreadcrumb],
    drag_active: bool,
    action: &mut ItemViewerNavBarAction,
) {
    if drag_active || !resp.clicked() {
        return;
    }

    if crumb.is_ellipsis {
        if let Some(first) = segments.first() {
            action.nav_to = Some(first.path.clone());
        }
    } else {
        action.nav_to = Some(crumb.path.clone());
    }
}

fn handle_breadcrumb_drag(
    ui: &egui::Ui,
    resp: &egui::Response,
    crumb: &RenderedBreadcrumb,
    segments: &[RenderedBreadcrumb],
    palette: &ThemePalette,
    drag_active: bool,
    hovered_target_ref: Option<&PathBuf>,
    pointer_pos: Option<egui::Pos2>,
    pointer_released: bool,
    breadcrumb_drop_target: &mut Option<PathBuf>,
    action: &mut ItemViewerNavBarAction,
) {
    if !drag_active {
        return;
    }

    let breadcrumb_target_path = if crumb.is_ellipsis {
        segments.first().map(|c| &c.path)
    } else {
        Some(&crumb.path)
    };

    let hovered = breadcrumb_target_path
        .and_then(|target_path| hovered_target_ref.map(|target| target == target_path))
        .unwrap_or_else(|| {
            pointer_pos
                .map(|pointer| resp.rect.contains(pointer))
                .unwrap_or(false)
        });

    if !hovered {
        return;
    }

    let painter = ui
        .ctx()
        .layer_painter(egui::LayerId::new(
            egui::Order::Background,
            ui.id().with("breadcrumb_drop_bg").with(&crumb.path),
        ))
        .with_clip_rect(ui.clip_rect());

    painter.rect_filled(
        resp.rect,
        egui::CornerRadius::same(palette.medium_radius),
        palette.primary_hover,
    );

    if pointer_released {
        let target = if crumb.is_ellipsis {
            segments.first().map(|c| c.path.clone())
        } else {
            Some(crumb.path.clone())
        };

        if let Some(target) = target {
            *breadcrumb_drop_target = Some(target);
            action.move_files_to_breadcrumb_dir_rect = Some(resp.rect);
        }
    }
}

pub(crate) fn merge_toolbar_action(
    action: &mut ItemViewerNavBarAction,
    toolbar: ItemViewerNavBarAction,
) {
    if action.nav.is_none() {
        action.nav = toolbar.nav;
    }

    if action.nav_to.is_none() {
        action.nav_to = toolbar.nav_to;
    }

    action.refresh_current_directory |= toolbar.refresh_current_directory;
    action.create_folder |= toolbar.create_folder;
    action.create_file |= toolbar.create_file;
    action.add_favorite |= toolbar.add_favorite;
    action.remove_favorite |= toolbar.remove_favorite;
    action.toggle_gallery |= toolbar.toggle_gallery;
    if action.context_action.is_none() {
        action.context_action = toolbar.context_action;
    }
    if action.rename_path.is_none() {
        action.rename_path = toolbar.rename_path;
    }
    if action.sort_by.is_none() {
        action.sort_by = toolbar.sort_by;
    }
}
