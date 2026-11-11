use crate::core::drives::{
    DriveInfo, consume_drive_list_dirty, get_drive_infos, is_raw_physical_drive_path,
};
use crate::core::fs::{MY_PC_PATH, MY_RECYCLE_BIN_PATH};
use crate::core::i18n::I18n;
use crate::core::icons::IconCache;
use crate::core::utils::text::apply_eden_text_overrides;
use crate::core::utils::widgets::request_animation_frame;
use crate::core::wsl::{list_distributions, wsl_path};
use crate::gui::customization::{SidebarPrefs, split_network_drives};
use crate::gui::folder_icons::{FolderIcon, paint_folder_icon, resolve_for_path};
use crate::gui::icon_system::{AppIcon, paint_icon};
use crate::gui::theme::ThemePalette;
use crate::gui::utils::{drive_usage_color, truncate_item_text};
use crate::gui::windows::containers::structs::SidebarAction;
use crate::gui::windows::structs::SidebarState;
use eframe::egui;
use egui::containers::{Popup, PopupCloseBehavior};
use egui::{FontFamily, FontId, ScrollArea};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Draw the sidebar, supporting favorites reordering
#[allow(clippy::too_many_arguments)]
pub fn draw_sidebar(
    ui: &mut egui::Ui,
    i18n: &I18n,
    icon_cache: &IconCache,
    sidebar_state: &mut SidebarState,
    palette: &ThemePalette,
    drag_active: bool,
    drag_hover_target: Option<PathBuf>,
    prefs: SidebarPrefs,
) -> SidebarAction {
    const DRIVE_CACHE_DURATION: Duration = Duration::from_secs(30);
    let mut action = SidebarAction::default();
    let pointer_pos = ui.ctx().input(|i| i.pointer.hover_pos());
    let pointer_released = ui.ctx().input(|i| i.pointer.primary_released());
    let mut tab_drop_target: Option<PathBuf> = None;
    let hovered_target_ref = drag_hover_target.as_ref();

    ScrollArea::vertical()
        .id_salt("sidebar_scroll")
        .auto_shrink([false; 2]) // don't shrink horizontally or vertically
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    ui.add_space(8.0);
                    ui.spacing_mut().item_spacing.y *= palette.sidebar_item_spacing_y;

                    if prefs.places {
                        section_title(ui, palette, prefs, &i18n.tr("places"));

                        let pc_icon_path = PathBuf::from("C:\\");
                        let resp = draw_sidebar_item(
                            ui,
                            icon_cache,
                            &pc_icon_path,
                            &i18n.tr("thispc"),
                            true,
                            false,
                            palette,
                            false,
                            None,
                        );
                        if resp.clicked() {
                            action.nav_to = Some(PathBuf::from(MY_PC_PATH));
                        }
                        if resp.middle_clicked() {
                            action.open_background_tab = Some(PathBuf::from(MY_PC_PATH));
                        }

                        Popup::context_menu(&resp)
                            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                            .show(|ui| {
                                apply_eden_text_overrides(ui, palette);
                                if ui.button(i18n.tr("inputs_newtab")).clicked() {
                                    action.open_new_tab = Some(PathBuf::from(MY_PC_PATH));
                                    ui.close();
                                }
                            });

                        if let Some(home) = dirs::home_dir() {
                            let resp = draw_sidebar_item(
                                ui,
                                icon_cache,
                                &home,
                                &i18n.tr("my_user_home"),
                                true,
                                false,
                                palette,
                                false,
                                None,
                            );

                            if resp.clicked() {
                                action.nav_to = Some(home.clone());
                            }
                            if resp.middle_clicked() {
                                action.open_background_tab = Some(home.clone());
                            }
                            Popup::context_menu(&resp)
                                .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                                .show(|ui| {
                                    apply_eden_text_overrides(ui, palette);
                                    if ui.button(i18n.tr("inputs_newtab")).clicked() {
                                        action.open_new_tab = Some(home.clone());
                                        ui.close();
                                    }
                                });
                        }

                        let recycle_bin_path = PathBuf::from("C:\\$Recycle.Bin");
                        let resp = draw_sidebar_item(
                            ui,
                            icon_cache,
                            &recycle_bin_path,
                            &i18n.tr("recycle_bin"),
                            true,
                            true,
                            palette,
                            false,
                            None,
                        );
                        if resp.clicked() {
                            action.nav_to = Some(PathBuf::from(MY_RECYCLE_BIN_PATH));
                        }
                        if resp.middle_clicked() {
                            action.open_background_tab = Some(PathBuf::from(MY_RECYCLE_BIN_PATH));
                        }

                        Popup::context_menu(&resp)
                            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                            .show(|ui| {
                                apply_eden_text_overrides(ui, palette);
                                if ui.button(i18n.tr("inputs_newtab")).clicked() {
                                    action.open_new_tab = Some(PathBuf::from(MY_RECYCLE_BIN_PATH));
                                    ui.close();
                                }
                            });
                    }

                    if prefs.quick_access {
                        section_gap(ui, prefs);
                        section_title(ui, palette, prefs, &i18n.tr("favorites"));

                        let count = sidebar_state.favorites.len();
                        let mut item_layouts = Vec::with_capacity(count);

                        // Layout pass: allocate every row at its resting position first so
                        // the reorder math sees the whole section before anything is drawn.
                        for (i, _favorite) in sidebar_state.favorites.iter().enumerate() {
                            let (rect, resp) = favorites_item_layout(ui, palette);

                            if !drag_active
                                && sidebar_state.dragging_favorite.is_none()
                                && resp.drag_started()
                            {
                                sidebar_state.dragging_favorite = Some(i);
                                sidebar_state.drag_grab_dy = resp
                                    .interact_pointer_pos()
                                    .map_or(rect.height() / 2.0, |p| p.y - rect.top());
                            }

                            item_layouts.push((rect, resp));
                        }

                        let section_top = item_layouts.first().map_or(0.0, |(r, _)| r.top());
                        let section_bottom = item_layouts.last().map_or(0.0, |(r, _)| r.bottom());
                        let row_height = item_layouts.first().map_or(18.0, |(r, _)| r.height());
                        let pitch = match item_layouts.as_slice() {
                            [(a, _), (b, _), ..] => b.top() - a.top(),
                            _ => row_height,
                        };

                        if sidebar_state
                            .dragging_favorite
                            .is_some_and(|from| from >= count)
                        {
                            sidebar_state.dragging_favorite = None;
                        } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            // Cancel: the row glides back from where it was.
                            if let Some(from) = sidebar_state.dragging_favorite.take() {
                                let back =
                                    (sidebar_state.drag_top - item_layouts[from].0.top()) / pitch;
                                let path = sidebar_state.favorites[from].path.clone();
                                sidebar_state.row_offsets.insert(path, back);
                            }
                        }

                        // Where the dragged row sits and which slot it would drop into.
                        let mut drag_top = sidebar_state.drag_top;
                        let mut target_slot = sidebar_state.dragging_favorite.unwrap_or(0);
                        if let (Some(_), Some(pos)) = (sidebar_state.dragging_favorite, pointer_pos)
                        {
                            drag_top = clamp_drag_top(
                                pos.y,
                                sidebar_state.drag_grab_dy,
                                section_top,
                                section_bottom,
                                row_height,
                            );
                            sidebar_state.drag_top = drag_top;
                            target_slot = favorite_target_slot(
                                drag_top + row_height / 2.0,
                                section_top,
                                pitch,
                                count,
                            );
                        }
                        if sidebar_state.dragging_favorite.is_some() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                        }

                        let dt = ui.input(|i| i.stable_dt).min(0.05);
                        let mut animating = false;
                        let favorites_len = count;
                        let rename_target = &mut sidebar_state.rename_target;
                        let rename_buffer = &mut sidebar_state.rename_buffer;
                        for (i, favorite) in sidebar_state.favorites.iter().enumerate() {
                            let (base_rect, resp) = &item_layouts[i];
                            let is_dragged = sidebar_state.dragging_favorite == Some(i);

                            // Slide toward the slot offset: neighbors open a gap at the target.
                            let goal = match sidebar_state.dragging_favorite {
                                Some(from) if !is_dragged => row_slot_offset(i, from, target_slot),
                                _ => 0.0,
                            };
                            let offset = if is_dragged {
                                0.0
                            } else {
                                let current = sidebar_state
                                    .row_offsets
                                    .get(&favorite.path)
                                    .copied()
                                    .unwrap_or(0.0);
                                let next = step_row_offset(current, goal, dt);
                                if next == 0.0 {
                                    sidebar_state.row_offsets.remove(&favorite.path);
                                } else {
                                    sidebar_state
                                        .row_offsets
                                        .insert(favorite.path.clone(), next);
                                    animating = true;
                                }
                                next
                            };

                            let rect = if is_dragged {
                                egui::Rect::from_min_size(
                                    egui::pos2(base_rect.left(), drag_top),
                                    base_rect.size(),
                                )
                            } else {
                                base_rect.translate(egui::vec2(0.0, offset * pitch))
                            };

                            // The dragged row lives on its own layer so it sits above the
                            // others, with an accent background and a soft shadow.
                            let mut overlay = is_dragged.then(|| {
                                let mut child = ui.new_child(
                                    egui::UiBuilder::new()
                                        .layer_id(egui::LayerId::new(
                                            egui::Order::Tooltip,
                                            ui.id().with("favorite_drag_row"),
                                        ))
                                        .max_rect(rect),
                                );
                                let radius = egui::CornerRadius::same(palette.medium_radius);
                                let painter = child.painter();
                                painter.add(
                                    egui::Shadow {
                                        offset: [0, 2],
                                        blur: 8,
                                        spread: 0,
                                        color: egui::Color32::from_black_alpha(70),
                                    }
                                    .as_shape(rect, radius),
                                );
                                painter.rect_filled(
                                    rect,
                                    radius,
                                    palette.primary_active.gamma_multiply(0.35),
                                );
                                child.set_clip_rect(rect.expand(16.0));
                                child
                            });
                            {
                                let row_ui: &mut egui::Ui = overlay.as_mut().unwrap_or(&mut *ui);
                                draw_sidebar_item(
                                    row_ui,
                                    icon_cache,
                                    &favorite.path,
                                    &favorite.label,
                                    true,
                                    false,
                                    palette,
                                    true,
                                    Some((rect, resp.clone())),
                                );
                                if prefs.pin_glyphs && !resp.hovered() {
                                    let pin_rect = egui::Rect::from_min_size(
                                        egui::pos2(rect.right() - 16.0, rect.top()),
                                        egui::vec2(12.0, rect.height()),
                                    );
                                    paint_icon(
                                        row_ui,
                                        pin_rect,
                                        AppIcon::PushPin.regular(),
                                        11.0,
                                        palette.text_normal.gamma_multiply(0.8),
                                    );
                                }
                            }

                            if drag_active {
                                let hovered = hovered_target_ref
                                    .map(|target| target == &favorite.path)
                                    .unwrap_or_else(|| {
                                        pointer_pos
                                            .map(|pointer| rect.contains(pointer))
                                            .unwrap_or(false)
                                    });
                                if hovered {
                                    let painter = ui
                                        .ctx()
                                        .layer_painter(egui::LayerId::new(
                                            egui::Order::Background,
                                            ui.id().with("tab_drop_bg").with(favorite.path.clone()),
                                        ))
                                        .with_clip_rect(ui.clip_rect());
                                    painter.rect_filled(
                                        rect,
                                        egui::CornerRadius::same(palette.medium_radius),
                                        palette.primary_hover,
                                    );

                                    if pointer_released {
                                        tab_drop_target = Some(favorite.path.clone());
                                    }
                                }
                            }

                            if resp.clicked() {
                                action.nav_to = Some(favorite.path.clone());
                            }
                            if resp.secondary_clicked() {
                                action.select_favorite = Some(favorite.path.clone());
                            }
                            if resp.middle_clicked() {
                                action.open_background_tab = Some(favorite.path.clone());
                            }

                            Popup::context_menu(resp)
                                .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                                .show(|ui| {
                                    apply_eden_text_overrides(ui, palette);
                                    if ui.button(i18n.tr("inputs_newtab")).clicked() {
                                        action.open_new_tab = Some(favorite.path.clone());
                                        ui.close();
                                    }
                                    if i > 0 && ui.button(i18n.tr("sidebar_move_up")).clicked() {
                                        action.move_favorite = Some((i, -1));
                                        ui.close();
                                    }
                                    if i + 1 < favorites_len
                                        && ui.button(i18n.tr("sidebar_move_down")).clicked()
                                    {
                                        action.move_favorite = Some((i, 1));
                                        ui.close();
                                    }
                                    ui.separator();
                                    if rename_target.as_ref() != Some(&favorite.path) {
                                        *rename_target = Some(favorite.path.clone());
                                        *rename_buffer = favorite.label.clone();
                                    }
                                    ui.horizontal(|ui| {
                                        let edit = ui.add(
                                            egui::TextEdit::singleline(rename_buffer)
                                                .desired_width(140.0)
                                                .hint_text(i18n.tr("sidebar_rename")),
                                        );
                                        let enter = edit.lost_focus()
                                            && ui.input(|i| i.key_pressed(egui::Key::Enter));
                                        if ui.button(i18n.tr("sidebar_rename")).clicked() || enter {
                                            action.rename_favorite = Some((
                                                favorite.path.clone(),
                                                rename_buffer.clone(),
                                            ));
                                            ui.close();
                                        }
                                    });
                                    ui.separator();
                                    if ui.button(i18n.tr("remove_favorite")).clicked() {
                                        action.remove_favorite = Some(favorite.path.clone());
                                        ui.close();
                                    }
                                });
                        }

                        // Drop: the row settles from where it was released into its slot.
                        if let Some(from) = sidebar_state.dragging_favorite
                            && pointer_released
                        {
                            let slot = target_slot.min(count.saturating_sub(1));
                            if slot != from {
                                for row in sidebar_state.row_offsets.values_mut() {
                                    *row = 0.0;
                                }
                                action.reorder = Some((from, reorder_insertion(from, slot)));
                            }
                            let settle = (drag_top - item_layouts[slot].0.top()) / pitch;
                            let path = sidebar_state.favorites[from].path.clone();
                            sidebar_state.row_offsets.insert(path, settle);
                            sidebar_state.dragging_favorite = None;
                            animating = true;
                        }
                        if animating {
                            request_animation_frame(ui.ctx());
                        }
                    }

                    if prefs.drives {
                        section_gap(ui, prefs);
                        section_title(ui, palette, prefs, &i18n.tr("storage"));
                    }

                    // Enumerate on a worker thread: a sleeping or disconnected drive can
                    // take long enough to stall a frame. One refresh runs at a time.
                    if sidebar_state.drive_refresh_rx.is_none()
                        && (sidebar_state.cached_drives.is_empty()
                            || sidebar_state.last_drive_refresh.elapsed() > DRIVE_CACHE_DURATION
                            || consume_drive_list_dirty())
                    {
                        let (tx, rx) = std::sync::mpsc::channel();
                        let ctx = ui.ctx().clone();
                        std::thread::spawn(move || {
                            let _ = tx.send(get_drive_infos());
                            ctx.request_repaint();
                        });
                        sidebar_state.drive_refresh_rx = Some(rx);
                    }
                    let polled = sidebar_state
                        .drive_refresh_rx
                        .as_ref()
                        .map(|rx| rx.try_recv());
                    match polled {
                        Some(Ok(drives)) => {
                            sidebar_state.cached_drives = drives;
                            sidebar_state.last_drive_refresh = Instant::now();
                            sidebar_state.drive_refresh_rx = None;
                        }
                        Some(Err(std::sync::mpsc::TryRecvError::Disconnected)) => {
                            sidebar_state.drive_refresh_rx = None;
                        }
                        Some(Err(std::sync::mpsc::TryRecvError::Empty)) | None => {}
                    }

                    let (local_drives, network_drives) =
                        split_network_drives(&sidebar_state.cached_drives, |d| {
                            d.is_network && prefs.network
                        });
                    let shown_local: &[&DriveInfo] = if prefs.drives { &local_drives } else { &[] };
                    let mut drive_rows: Vec<&DriveInfo> = shown_local.to_vec();
                    let network_start = drive_rows.len();
                    drive_rows.extend(network_drives.iter().copied());
                    for (row, drive) in drive_rows.into_iter().enumerate() {
                        if row == network_start {
                            section_gap(ui, prefs);
                            section_title(ui, palette, prefs, &i18n.tr("sidebar_network"));
                        }
                        let is_selected = sidebar_state
                            .item_clicked
                            .as_ref()
                            .map(|p| p == &drive.path)
                            .unwrap_or(false);

                        let resp = sidebar_drive_item(
                            ui,
                            icon_cache,
                            drive,
                            palette,
                            is_selected,
                            prefs.drive_usage_bars,
                        );
                        if resp.clicked() {
                            if is_raw_physical_drive_path(&drive.path) {
                                sidebar_state.non_ntfs_popup_path = Some(drive.path.clone());
                            } else {
                                action.nav_to = Some(drive.path.clone());
                            }
                        }
                        if resp.middle_clicked() {
                            if is_raw_physical_drive_path(&drive.path) {
                                sidebar_state.non_ntfs_popup_path = Some(drive.path.clone());
                            } else {
                                action.open_background_tab = Some(drive.path.clone());
                            }
                        }
                    }

                    if prefs.linux {
                        let distros = sidebar_state
                            .wsl_distros
                            .get_or_insert_with(list_distributions);
                        if !distros.is_empty() {
                            section_gap(ui, prefs);
                            section_title(ui, palette, prefs, &i18n.tr("sidebar_linux"));
                            for distro in distros.iter() {
                                let path = wsl_path(distro);
                                let resp = draw_sidebar_item(
                                    ui, icon_cache, &path, distro, true, false, palette, false,
                                    None,
                                );
                                if resp.clicked() {
                                    action.nav_to = Some(path.clone());
                                }
                                if resp.middle_clicked() {
                                    action.open_background_tab = Some(path.clone());
                                }
                            }
                        }
                    }

                    if let Some(_path) = sidebar_state.non_ntfs_popup_path.clone() {
                        let mut open = true;
                        egui::Window::new(i18n.tr("non_nftsdrive"))
                            .collapsible(false)
                            .resizable(false)
                            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                            .open(&mut open)
                            .show(ui.ctx(), |ui| {
                                ui.label(i18n.tr("non_nftsdrive_label1"));
                                ui.label(i18n.tr("non_nftsdrive_label2"));
                                ui.label(i18n.tr("non_nftsdrive_label3"));
                                ui.add_space(8.0);
                                if ui.button(i18n.tr("ok")).clicked() {
                                    sidebar_state.non_ntfs_popup_path = None;
                                }
                            });
                        if !open {
                            sidebar_state.non_ntfs_popup_path = None;
                        }
                    }

                    // Extra bottom padding so last items aren't clipped by scroll boundary.
                    ui.add_space(12.0);
                });
                ui.add_space(2.0);
            });
        });

    action.move_files_to_sidebar_dir = tab_drop_target;
    action
}

pub fn draw_sidebar_item(
    ui: &mut egui::Ui,
    icon_cache: &IconCache,
    path: &std::path::Path,
    label: &str,
    is_dir: bool,
    is_recycle_bin: bool,
    palette: &ThemePalette,
    draggable: bool,
    rect_and_resp: Option<(egui::Rect, egui::Response)>,
) -> egui::Response {
    let height = if palette.sidebar_item_spacing_y > 1.0 {
        18.0 * palette.sidebar_item_spacing_y
    } else {
        18.0
    };

    // --- Get rect + response ---
    let (rect, resp) = if let Some((rect, resp)) = rect_and_resp {
        (rect, resp)
    } else {
        let available_width = ui.available_width();
        ui.allocate_exact_size(
            egui::vec2(available_width, height),
            if draggable {
                egui::Sense::click_and_drag()
            } else {
                egui::Sense::click()
            },
        )
    };

    // --- Hover background + cursor ---
    if resp.hovered() {
        ui.ctx().set_cursor_icon(if draggable {
            egui::CursorIcon::Grab
        } else {
            egui::CursorIcon::Default
        });

        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same(palette.medium_radius),
            palette.primary_hover,
        );

        if draggable {
            let handle_width = 12.0;
            let handle_rect = egui::Rect::from_min_size(
                egui::pos2(rect.right() - handle_width - 4.0, rect.top()),
                egui::vec2(handle_width, rect.height()),
            );

            paint_icon(
                ui,
                handle_rect,
                AppIcon::DotsSixVertical.regular(),
                14.0,
                palette.icon_color,
            );
        }
    }

    // --- Icon ---
    let scaled_sidebar_icon_size = palette.sidebar_icon_size * (palette.text_size / 12.0);
    let icon_size = egui::vec2(scaled_sidebar_icon_size, scaled_sidebar_icon_size);
    let icon_padding = 4.0;

    let icon_pos = egui::pos2(rect.min.x + 4.0, rect.center().y - icon_size.y / 2.0);

    let text_offset_x = if is_recycle_bin {
        paint_icon(
            ui,
            egui::Rect::from_min_size(icon_pos, icon_size),
            AppIcon::Trash.regular(),
            icon_size.y * 0.85,
            palette.icon_color,
        );

        palette.text_size + icon_size.x + icon_padding
    } else if paint_folder_icon(
        ui,
        egui::Rect::from_min_size(icon_pos, icon_size),
        icon_size.y,
        sidebar_folder_icon(palette, path, is_dir),
        false,
    ) {
        // Theme-styled icon for folders (glyph for known folders, flat folder).
        palette.text_size + icon_size.x + icon_padding
    } else if let Some(icon) = icon_cache.get(path, is_dir) {
        // Windows shell icon fallback.
        ui.painter().image(
            (&icon).into(),
            egui::Rect::from_min_size(icon_pos, icon_size),
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1.0, 1.0)),
            egui::Color32::WHITE,
        );

        palette.text_size + icon_size.x + icon_padding
    } else {
        palette.text_size + 20.0 + icon_padding
    };

    // --- Text ---
    let text_width = rect.width() - text_offset_x;
    let font_id = egui::FontId::new(palette.text_size, egui::FontFamily::Proportional);
    let color = ui.visuals().text_color();

    let (display_name, truncated) = truncate_item_text(ui, label, text_width, &font_id, color);

    let text_pos = egui::pos2(rect.min.x + text_offset_x, rect.center().y - 2.0);

    ui.painter().text(
        text_pos,
        egui::Align2::LEFT_CENTER,
        display_name,
        font_id,
        color,
    );

    // --- Tooltip + cursor ---
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);

    if truncated {
        resp.on_hover_text(
            egui::RichText::new(label)
                .size(palette.tooltip_text_size)
                .color(palette.tooltip_text_color),
        )
    } else {
        resp.on_hover_text(
            egui::RichText::new(path.to_string_lossy())
                .size(palette.tooltip_text_size)
                .color(palette.tooltip_text_color),
        )
    }
}

/// The sidebar used `icon_color` for known-folder glyphs; keep that.
fn sidebar_folder_icon(palette: &ThemePalette, path: &std::path::Path, is_dir: bool) -> FolderIcon {
    match resolve_for_path(palette, path, is_dir) {
        FolderIcon::Glyph(glyph, _) => FolderIcon::Glyph(glyph, palette.icon_color),
        other => other,
    }
}

/// Bold section title, unless titles are turned off.
fn section_title(ui: &mut egui::Ui, palette: &ThemePalette, prefs: SidebarPrefs, text: &str) {
    if !prefs.section_titles {
        return;
    }
    ui.add(egui::Label::new(
        egui::RichText::new(text)
            .size(palette.text_size)
            .color(palette.text_header_section)
            .strong(),
    ));
    ui.add_space(4.0);
}

/// Space between sections; a thin divider when titles are hidden (Explorer).
fn section_gap(ui: &mut egui::Ui, prefs: SidebarPrefs) {
    ui.add_space(6.0);
    if !prefs.section_titles {
        ui.separator();
        ui.add_space(2.0);
    }
}

/// Top edge of the dragged favorite: follows the pointer (minus the grab point)
/// and stays inside the favorites section.
fn clamp_drag_top(pointer_y: f32, grab_dy: f32, top: f32, bottom: f32, row_height: f32) -> f32 {
    let max = (bottom - row_height).max(top);
    (pointer_y - grab_dy).clamp(top, max)
}

/// Slot (0..count) the dragged row's center falls in.
fn favorite_target_slot(center_y: f32, top: f32, pitch: f32, count: usize) -> usize {
    if count == 0 || pitch <= 0.0 {
        return 0;
    }
    (((center_y - top) / pitch).floor().max(0.0) as usize).min(count - 1)
}

/// Offset in rows (-1, 0 or 1) a non-dragged row takes so a gap opens at `target`.
fn row_slot_offset(index: usize, drag: usize, target: usize) -> f32 {
    if drag < target && index > drag && index <= target {
        -1.0
    } else if target < drag && index >= target && index < drag {
        1.0
    } else {
        0.0
    }
}

/// Insertion index for `SidebarAction::reorder` when moving `from` into `slot`.
fn reorder_insertion(from: usize, slot: usize) -> usize {
    if slot > from { slot + 1 } else { slot }
}

/// Ease-out step (about 140 ms to settle) of a row offset toward its goal.
fn step_row_offset(current: f32, goal: f32, dt: f32) -> f32 {
    const TIME_CONSTANT: f32 = 0.04;
    let next = current + (goal - current) * (1.0 - (-dt / TIME_CONSTANT).exp());
    if (goal - next).abs() < 0.01 {
        goal
    } else {
        next
    }
}

fn favorites_item_layout(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
) -> (egui::Rect, egui::Response) {
    let available_width = ui.available_width();

    let height = if palette.sidebar_item_spacing_y > 1.0 {
        18.0 * palette.sidebar_item_spacing_y
    } else {
        18.0
    };

    ui.allocate_exact_size(
        egui::vec2(available_width, height),
        egui::Sense::click_and_drag(),
    )
}

/// Draw a drive item with usage bar and size on hover
fn sidebar_drive_item(
    ui: &mut egui::Ui,
    icon_cache: &IconCache,
    drive: &DriveInfo,
    palette: &ThemePalette,
    selected: bool,
    usage_bar: bool,
) -> egui::Response {
    let available_width = ui.available_width();
    let base_height = if usage_bar { 32.0 } else { 18.0 };
    let height = if palette.sidebar_item_spacing_y > 1.0 {
        base_height * palette.sidebar_item_spacing_y
    } else {
        base_height
    };

    let (rect, mut resp) =
        ui.allocate_exact_size(egui::vec2(available_width, height), egui::Sense::click());

    // Background (selected > active click > hover)
    let fill_color = if selected || resp.is_pointer_button_down_on() {
        palette.primary_active
    } else if resp.hovered() {
        palette.primary_hover
    } else {
        egui::Color32::TRANSPARENT
    };

    if fill_color != egui::Color32::TRANSPARENT {
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same(palette.medium_radius),
            fill_color,
        );
    }

    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    // --- Top row: icon + label ---
    let scaled_sidebar_icon_size = palette.sidebar_icon_size * (palette.text_size / 12.0);
    let icon_size = egui::vec2(scaled_sidebar_icon_size, scaled_sidebar_icon_size);
    let icon_padding = 4.0;

    let text_offset_x = if let Some(icon) = icon_cache.get(&drive.path, true) {
        let icon_pos = egui::pos2(rect.min.x + 4.0, rect.min.y + 4.0);

        ui.painter().image(
            (&icon).into(),
            egui::Rect::from_min_size(icon_pos, icon_size),
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1.0, 1.0)),
            palette.icon_windows,
        );

        palette.text_size + icon_size.x + icon_padding
    } else {
        palette.text_size + 20.0 + icon_padding
    };

    // --- DISPLAY TEXT ---
    let text_width = available_width - text_offset_x;
    let max_chars = (text_width / 7.0) as usize;

    let display_name = if drive.display.len() > max_chars && max_chars > 3 {
        // Use character boundaries instead of byte indices
        let mut byte_end = 0;
        for (char_count, (i, _)) in drive.display.char_indices().enumerate() {
            if char_count >= max_chars - 3 {
                break;
            }
            byte_end = i;
        }
        format!("{}...", &drive.display[..byte_end])
    } else {
        drive.display.clone()
    };

    let text_y = if usage_bar {
        rect.min.y + 4.0 + icon_size.y / 2.0
    } else {
        rect.center().y
    };
    let text_pos = egui::pos2(rect.min.x + text_offset_x, text_y);
    let font_id = FontId::new(palette.text_size, FontFamily::Proportional);

    ui.painter().text(
        text_pos,
        egui::Align2::LEFT_CENTER,
        display_name,
        font_id,
        ui.visuals().text_color(),
    );

    // --- Bottom row: progress bar ---
    if let (true, Some(total), Some(free)) = (usage_bar, drive.total_space, drive.free_space) {
        let bar_height = 6.0;
        let max_bar_width = 180.0;
        let bar_width = (available_width - 8.0).min(max_bar_width);

        let bar_rect = egui::Rect::from_min_size(
            egui::pos2(rect.min.x + 4.0, rect.bottom() - bar_height),
            egui::vec2(bar_width, bar_height),
        );

        let bar_bg = palette.drive_usage_background;
        let bar_fill = drive_usage_color((total - free) as f32 / total as f32, palette);

        ui.painter().rect_filled(
            bar_rect,
            egui::CornerRadius::same(palette.small_radius),
            bar_bg,
        );

        let used_ratio = (total - free) as f32 / total as f32;
        let fill_width = bar_rect.width() * used_ratio;

        let fill_rect = egui::Rect::from_min_size(bar_rect.min, egui::vec2(fill_width, bar_height));

        ui.painter().rect_filled(
            fill_rect,
            egui::CornerRadius::same(palette.small_radius),
            bar_fill,
        );

        let gb = 1024.0 * 1024.0 * 1024.0;
        let used_gb = (total - free) as f64 / gb;
        let total_gb = total as f64 / gb;

        resp = resp.on_hover_text(
            egui::RichText::new(format!("{:.1}/{:.1}GB", used_gb, total_gb))
                .size(palette.tooltip_text_size)
                .color(palette.tooltip_text_color),
        );
    }

    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_top_is_clamped_to_section() {
        assert_eq!(clamp_drag_top(50.0, 5.0, 100.0, 200.0, 20.0), 100.0);
        assert_eq!(clamp_drag_top(500.0, 5.0, 100.0, 200.0, 20.0), 180.0);
        assert_eq!(clamp_drag_top(150.0, 5.0, 100.0, 200.0, 20.0), 145.0);
    }

    #[test]
    fn target_slot_follows_center_and_clamps() {
        assert_eq!(favorite_target_slot(95.0, 100.0, 20.0, 5), 0);
        assert_eq!(favorite_target_slot(119.0, 100.0, 20.0, 5), 0);
        assert_eq!(favorite_target_slot(121.0, 100.0, 20.0, 5), 1);
        assert_eq!(favorite_target_slot(900.0, 100.0, 20.0, 5), 4);
        assert_eq!(favorite_target_slot(120.0, 100.0, 20.0, 0), 0);
    }

    #[test]
    fn rows_open_a_gap_at_the_target() {
        // Dragging 1 down to 3: rows 2 and 3 move up.
        let down: Vec<f32> = (0..5).map(|i| row_slot_offset(i, 1, 3)).collect();
        assert_eq!(down, [0.0, 0.0, -1.0, -1.0, 0.0]);
        // Dragging 3 up to 1: rows 1 and 2 move down.
        let up: Vec<f32> = (0..5).map(|i| row_slot_offset(i, 3, 1)).collect();
        assert_eq!(up, [0.0, 1.0, 1.0, 0.0, 0.0]);
        // Same slot: nothing moves.
        assert!((0..5).all(|i| row_slot_offset(i, 2, 2) == 0.0));
    }

    #[test]
    fn reorder_insertion_matches_remove_then_insert() {
        assert_eq!(reorder_insertion(1, 3), 4);
        assert_eq!(reorder_insertion(3, 1), 1);
    }

    #[test]
    fn row_offset_eases_to_goal() {
        let mut v = 1.0;
        for _ in 0..9 {
            v = step_row_offset(v, 0.0, 1.0 / 60.0);
        }
        assert!(v > 0.0 && v < 0.1);
        for _ in 0..20 {
            v = step_row_offset(v, 0.0, 1.0 / 60.0);
        }
        assert_eq!(v, 0.0);
    }
}
