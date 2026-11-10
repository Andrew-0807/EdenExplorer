use crate::core::i18n::I18n;
use crate::core::{
    fs::{DateStyle, MY_PC_PATH},
    indexer::{
        WindowSizeMode, default_item_viewer_drive_column_size,
        default_item_viewer_file_column_size, default_recycle_bin_column_size,
    },
    utils::text::apply_eden_text_overrides,
    utils::widgets::{
        apply_eden_visual_overrides, draw_dropdown_sized, eden_button, eden_icon_button,
        eden_text_label, paint_focus_ring,
    },
};
use crate::gui::customization::{
    CheckboxVisibility, SidebarPrefs, StatusBarPrefs, ToolbarLayout, ToolbarPrefs,
};
use crate::gui::icon_system::AppIcon;
use crate::gui::theme::ThemeMode;
use crate::gui::theme::ThemePalette;
use crate::gui::utils::SortColumn;
use crate::gui::windows::containers::enums::ItemViewerHeaderColumn;
use crate::gui::windows::enums::SettingsAction;
use crate::gui::windows::hotkey_editor;
use crate::gui::windows::structs::{AppSettings, SettingsTab, SettingsWindow, ThemeCustomizer};
use crate::gui::windows::windowsoverrides::clamp_dialog_size;
use eframe::egui;
use egui::RichText;
use std::path::PathBuf;

mod sections;
mod theme;

/// Space between the label column and the control column.
const SETTINGS_CONTROL_GAP: f32 = 12.0;
const SETTINGS_COMBO_WIDTH: f32 = 180.0;
const SETTINGS_VALUE_WIDTH: f32 = 90.0;
/// Width of a slider control, track plus its value box.
const SETTINGS_SLIDER_WIDTH: f32 = 220.0;
/// Width of the left section list.
const SETTINGS_NAV_WIDTH: f32 = 180.0;
const SETTINGS_WINDOW_DEFAULT_SIZE: [f32; 2] = [820.0, 600.0];
const SETTINGS_WINDOW_MIN_SIZE: [f32; 2] = [600.0, 420.0];

/// Text size of settings labels and controls: the app's own text size.
pub(crate) fn body_size(palette: &ThemePalette) -> f32 {
    palette.text_size
}

/// Muted description line under a row name.
fn small_size(palette: &ThemePalette) -> f32 {
    (palette.text_size - 2.0).max(8.0)
}

/// Height of combo buttons, drag values and push buttons.
fn control_height(palette: &ThemePalette) -> f32 {
    (palette.text_size + 12.0).max(24.0)
}

/// Click target and drawn track of a toggle switch.
fn toggle_hit(palette: &ThemePalette) -> egui::Vec2 {
    egui::vec2(control_height(palette) * 1.6, control_height(palette))
}

fn toggle_track(palette: &ThemePalette) -> egui::Vec2 {
    let h = (palette.text_size + 6.0).max(16.0);
    egui::vec2((h * 1.8).round(), h)
}

/// Height of one row: a name line, plus a description line when there is one.
fn row_height(palette: &ThemePalette, has_description: bool) -> f32 {
    let name = body_size(palette) * 1.3;
    if has_description {
        name + small_size(palette) * 1.3 + 16.0
    } else {
        (name + 16.0).max(control_height(palette) + 8.0)
    }
}

/// Square size of icon buttons.
fn icon_button_size(palette: &ThemePalette) -> f32 {
    control_height(palette)
}

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
            zoxide_auto_add: false,
            hotkeys: Default::default(),
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

/// What the search box is filtering this frame. Rows and buttons ask `search_admits` before
/// drawing, so every tab filters itself without per-tab code. `shown` counts the items that
/// were admitted, which lets sections and categories hide when nothing in them matched.
#[derive(Clone, Default)]
struct SearchFilter {
    query: String,
    shown: usize,
}

fn search_filter_id() -> egui::Id {
    egui::Id::new("settings_search_filter")
}

fn search_field_id() -> egui::Id {
    egui::Id::new("settings_search_field")
}

/// Starts a frame: sets the query and resets the match counter.
fn begin_search_frame(ctx: &egui::Context, query: &str) {
    ctx.data_mut(|data| {
        data.insert_temp(
            search_filter_id(),
            SearchFilter {
                query: query.trim().to_string(),
                shown: 0,
            },
        );
    });
}

fn search_active(ctx: &egui::Context) -> bool {
    ctx.data(|data| {
        data.get_temp::<SearchFilter>(search_filter_id())
            .is_some_and(|filter| !filter.query.is_empty())
    })
}

fn search_shown_count(ctx: &egui::Context) -> usize {
    ctx.data(|data| {
        data.get_temp::<SearchFilter>(search_filter_id())
            .map_or(0, |filter| filter.shown)
    })
}

/// True when an item with these search texts should be drawn (always, without a query).
/// Counts the item as shown.
fn search_admits(ctx: &egui::Context, texts: &[&str]) -> bool {
    ctx.data_mut(|data| {
        let filter = data.get_temp_mut_or_default::<SearchFilter>(search_filter_id());
        let admitted = matches_search(&filter.query, texts);
        if admitted {
            filter.shown += 1;
        }
        admitted
    })
}

/// Every whitespace-separated term of `query` must appear (case-insensitive) in at least one
/// of `texts`. An empty query matches everything.
fn matches_search(query: &str, texts: &[&str]) -> bool {
    let mut terms = query.split_whitespace().map(str::to_lowercase).peekable();
    if terms.peek().is_none() {
        return true;
    }
    let haystacks: Vec<String> = texts.iter().map(|text| text.to_lowercase()).collect();
    terms.all(|term| haystacks.iter().any(|haystack| haystack.contains(&term)))
}

/// Lays out `add_contents` in `ui` only if it drew an item that passed the search filter
/// (always, when no search is active). The contents run in a child ui, so a skipped one
/// takes no space.
fn show_if_matches<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let ctx = ui.ctx().clone();
    let shown_before = search_shown_count(&ctx);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(ui.available_rect_before_wrap())
            .layout(*ui.layout()),
    );
    let result = add_contents(&mut child);
    if !search_active(&ctx) || search_shown_count(&ctx) > shown_before {
        ui.allocate_rect(child.min_rect(), egui::Sense::hover());
    }
    result
}

/// Optional heading plus flat rows; hidden entirely when a search matches none of them.
fn settings_section(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    heading: Option<&str>,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    show_if_matches(ui, |ui| {
        if let Some(heading) = heading {
            settings_heading(ui, palette, heading);
        }
        add_contents(ui);
    });
}

/// Small bold heading above a group of rows.
fn settings_heading(ui: &mut egui::Ui, palette: &ThemePalette, text: &str) {
    ui.add_space(14.0);
    ui.label(
        RichText::new(text)
            .size(body_size(palette))
            .strong()
            .color(palette.text_header_section),
    );
    ui.add_space(4.0);
    divider(ui, palette);
}

/// Thin line between rows.
fn divider(ui: &mut egui::Ui, palette: &ThemePalette) {
    let rect = ui.available_rect_before_wrap();
    ui.painter().hline(
        rect.x_range(),
        rect.top(),
        egui::Stroke::new(1.0, palette.borders_default.gamma_multiply(0.6)),
    );
    ui.add_space(1.0);
}

/// Collapsible "Advanced" block at the bottom of a section. Always open while searching so its
/// rows can match.
fn settings_advanced(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    i18n: &I18n,
    id: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    settings_collapsible_with(ui, palette, &i18n.tr("settings_advanced"), id, add_contents);
}

/// A collapsible block titled by an i18n key.
fn settings_collapsible(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    i18n: &I18n,
    key: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    settings_collapsible_with(ui, palette, &i18n.tr(key), key, add_contents);
}

fn settings_collapsible_with(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    title: &str,
    id: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    show_if_matches(ui, |ui| {
        ui.add_space(14.0);
        let title = RichText::new(title)
            .size(body_size(palette))
            .strong()
            .color(palette.text_header_section);
        let mut header = egui::CollapsingHeader::new(title)
            .id_salt(("settings_collapsible", id))
            .default_open(false);
        if search_active(ui.ctx()) {
            header = header.open(Some(true));
        }
        header.show(ui, add_contents);
    });
}

/// Left list entry: subtle fill when selected, lighter tint on hover.
fn nav_item(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    selected: bool,
    text: &str,
) -> egui::Response {
    let height = control_height(palette) + 2.0;
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::click(),
    );
    let radius = egui::CornerRadius::same(palette.medium_radius);
    if selected {
        ui.painter().rect_filled(rect, radius, palette.widget_bg);
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, radius, palette.faint_bg);
    }
    ui.painter().text(
        egui::pos2(rect.left() + 10.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        egui::FontId::proportional(body_size(palette)),
        if selected {
            palette.text_header_section
        } else {
            palette.text_normal
        },
    );
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, text)
    });
    paint_focus_ring(ui, rect, &resp, radius);
    resp
}

/// Small muted header above a group of sidebar entries.
fn nav_group_header(ui: &mut egui::Ui, palette: &ThemePalette, text: &str) {
    ui.add_space(8.0);
    ui.label(
        RichText::new(text)
            .size(small_size(palette))
            .color(palette.text_normal.gamma_multiply(0.6)),
    );
    ui.add_space(2.0);
}

/// Toggle switch bound to a bool: accent track when on, sliding knob. Click target is
/// `toggle_hit`; the drawn track is smaller and centered in it.
fn draw_toggle_switch(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    on: &mut bool,
    id: impl std::hash::Hash + std::fmt::Debug,
    label: &str,
) -> egui::Response {
    let (cell, _) = ui.allocate_exact_size(toggle_hit(palette), egui::Sense::hover());
    let mut resp = ui.interact(cell, ui.id().with(id), egui::Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *on, label)
    });

    let t = ui.ctx().animate_bool_responsive(resp.id, *on);
    let track_size = toggle_track(palette);
    let track = egui::Rect::from_center_size(cell.center(), track_size);
    let off_fill = if resp.hovered() {
        palette.checkbox_bg_hover
    } else {
        palette.checkbox_bg_default
    };
    let on_fill = if resp.hovered() {
        palette.primary_hover
    } else {
        palette.primary
    };
    let radius = (track_size.y / 2.0).round() as u8;
    ui.painter().rect(
        track,
        egui::CornerRadius::same(radius),
        off_fill.lerp_to_gamma(on_fill, t),
        egui::Stroke::new(1.0, palette.borders_default.lerp_to_gamma(on_fill, t)),
        egui::StrokeKind::Inside,
    );
    let knob_radius = track_size.y / 2.0 - 3.0;
    let travel = track_size.x / 2.0 - knob_radius - 3.0;
    ui.painter().circle_filled(
        egui::pos2(
            track.center().x + travel * (2.0 * t - 1.0),
            track.center().y,
        ),
        knob_radius,
        palette
            .text_normal
            .lerp_to_gamma(palette.checkbox_checkmark_color, t),
    );
    paint_focus_ring(ui, track, &resp, egui::CornerRadius::same(radius));
    resp
}

/// Push button; `None` when the active search filters it out.
fn settings_button(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    text: &str,
) -> Option<egui::Response> {
    if !search_admits(ui.ctx(), &[text]) {
        return None;
    }
    Some(
        ui.add(
            egui::Button::new(
                RichText::new(text)
                    .size(body_size(palette))
                    .color(palette.text_normal),
            )
            .min_size(egui::vec2(0.0, control_height(palette))),
        ),
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
        icon_button_size(palette),
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
        Some(control_height(palette)),
        selected_text,
        add_contents,
    );
}

/// A toggle row: name and description left, switch right. Clicking the name toggles too.
/// True when it changed.
fn setting_toggle(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    checked: &mut bool,
    label: &str,
    description: Option<&str>,
    id: impl std::hash::Hash + std::fmt::Debug,
) -> bool {
    let mut changed = false;
    let label_resp = setting_row(
        ui,
        palette,
        label,
        description,
        toggle_hit(palette).x,
        |ui| {
            changed |= draw_toggle_switch(ui, palette, checked, id, label).clicked();
        },
    );
    if label_resp.is_some_and(|resp| resp.clicked()) {
        *checked = !*checked;
        changed = true;
    }
    changed
}

/// One settings row: name (and a muted one-line description under it) on the left, `control`
/// right-aligned in a column of `control_width`. Long text is cut with an ellipsis instead of
/// running under the control. Returns the name's response, or `None` when the active search
/// filters the row out (its `control` does not run).
pub fn setting_row(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    label: &str,
    description: Option<&str>,
    control_width: f32,
    control: impl FnOnce(&mut egui::Ui),
) -> Option<egui::Response> {
    setting_row_keywords(ui, palette, label, description, "", control_width, control)
}

/// `setting_row` with extra search text that is not shown (e.g. a shortcut's key combo).
pub fn setting_row_keywords(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    label: &str,
    description: Option<&str>,
    keywords: &str,
    control_width: f32,
    control: impl FnOnce(&mut egui::Ui),
) -> Option<egui::Response> {
    if !search_admits(ui.ctx(), &[label, description.unwrap_or(""), keywords]) {
        return None;
    }
    let description = description.filter(|d| !d.is_empty());
    let height = row_height(palette, description.is_some());
    let text_height =
        body_size(palette) * 1.3 + description.map_or(0.0, |_| small_size(palette) * 1.3);
    let mut label_resp = None;
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), height),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(control_width, height),
                egui::Layout::right_to_left(egui::Align::Center),
                control,
            );
            ui.add_space(SETTINGS_CONTROL_GAP);
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.add_space(((height - text_height) / 2.0).max(0.0));
                    let name = ui.add(
                        egui::Label::new(
                            RichText::new(label)
                                .size(body_size(palette))
                                .color(palette.text_normal),
                        )
                        .truncate()
                        .sense(egui::Sense::click()),
                    );
                    if let Some(description) = description {
                        ui.add(
                            egui::Label::new(
                                RichText::new(description)
                                    .size(small_size(palette))
                                    .color(palette.text_normal.gamma_multiply(0.6)),
                            )
                            .truncate(),
                        )
                        .on_hover_text(description);
                    }
                    label_resp = Some(name);
                },
            );
        },
    );
    divider(ui, palette);
    label_resp
}

/// Row with one push button on the right; true when clicked.
fn button_row(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    label: &str,
    description: Option<&str>,
    button: &str,
) -> bool {
    let mut clicked = false;
    setting_row(ui, palette, label, description, 96.0, |ui| {
        clicked = ui
            .add(
                egui::Button::new(
                    RichText::new(button)
                        .size(body_size(palette))
                        .color(palette.text_normal),
                )
                .min_size(egui::vec2(88.0, control_height(palette))),
            )
            .clicked();
    });
    clicked
}

/// Row with a number box you can drag or type into.
fn drag_row(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    label: &str,
    description: Option<&str>,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    speed: f64,
) -> bool {
    let mut changed = false;
    setting_row(
        ui,
        palette,
        label,
        description,
        SETTINGS_VALUE_WIDTH,
        |ui| {
            changed = ui
                .add_sized(
                    [SETTINGS_VALUE_WIDTH, control_height(palette)],
                    egui::DragValue::new(value).range(range).speed(speed),
                )
                .changed();
        },
    );
    changed
}

fn byte_row(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    label: &str,
    value: &mut u8,
    max: u8,
) -> bool {
    let mut changed = false;
    setting_row(ui, palette, label, None, SETTINGS_VALUE_WIDTH, |ui| {
        changed = ui
            .add_sized(
                [SETTINGS_VALUE_WIDTH, control_height(palette)],
                egui::DragValue::new(value).range(0..=max).speed(0.1),
            )
            .changed();
    });
    changed
}

/// Row with a slider and its value.
fn slider_row(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    label: &str,
    description: Option<&str>,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    suffix: &str,
    step: f64,
) -> bool {
    let mut changed = false;
    setting_row(
        ui,
        palette,
        label,
        description,
        SETTINGS_SLIDER_WIDTH,
        |ui| {
            ui.spacing_mut().slider_width = SETTINGS_SLIDER_WIDTH - 70.0;
            changed = ui
                .add(
                    egui::Slider::new(value, range)
                        .suffix(suffix)
                        .step_by(step)
                        .trailing_fill(true),
                )
                .changed();
        },
    );
    changed
}

/// Row with a color swatch button.
fn color_row(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    label: &str,
    description: Option<&str>,
    color: &mut egui::Color32,
) -> bool {
    let mut changed = false;
    setting_row(
        ui,
        palette,
        label,
        description,
        control_height(palette) * 2.0,
        |ui| {
            changed = crate::gui::utils::rgba_color_edit_button(ui, color).changed();
        },
    );
    changed
}

/// Row with a drop-down over `options` (value, i18n key); the description is an i18n key.
fn choice_row<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    i18n: &I18n,
    label_key: &str,
    description_key: Option<&str>,
    value: &mut T,
    options: &[(T, &str)],
) -> bool {
    let description = description_key.map(|key| i18n.tr(key));
    choice_row_desc(
        ui,
        palette,
        i18n,
        label_key,
        description.as_deref(),
        value,
        options,
    )
}

fn choice_row_desc<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    i18n: &I18n,
    label_key: &str,
    description: Option<&str>,
    value: &mut T,
    options: &[(T, &str)],
) -> bool {
    let mut changed = false;
    let current = options
        .iter()
        .find(|(v, _)| v == value)
        .map(|(_, k)| i18n.tr(k))
        .unwrap_or_default();
    setting_row(
        ui,
        palette,
        &i18n.tr(label_key),
        description,
        SETTINGS_COMBO_WIDTH,
        |ui| {
            settings_dropdown(
                ui,
                palette,
                ("choice", label_key),
                SETTINGS_COMBO_WIDTH,
                current,
                |ui| {
                    for (option, key) in options {
                        if ui.selectable_label(option == value, i18n.tr(key)).clicked() {
                            *value = *option;
                            changed = true;
                            ui.close();
                        }
                    }
                },
            );
        },
    );
    changed
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

/// Sidebar groups: a muted header and the sections under it.
const SETTINGS_GROUPS: [(&str, &[(SettingsTab, &str)]); 2] = [
    (
        "settings_group_options",
        &[
            (SettingsTab::General, "settings_tab_general"),
            (SettingsTab::Files, "settings_tab_files"),
            (SettingsTab::Interface, "settings_tab_interface"),
            (SettingsTab::Appearance, "settings_tab_appearance"),
        ],
    ),
    (
        "settings_group_more",
        &[
            (SettingsTab::Hotkeys, "settings_tab_hotkeys"),
            (SettingsTab::Integrations, "settings_tab_integrations"),
        ],
    ),
];

/// Every section in sidebar order.
fn settings_tabs() -> impl Iterator<Item = (SettingsTab, &'static str)> {
    SETTINGS_GROUPS
        .iter()
        .flat_map(|(_, tabs)| tabs.iter().copied())
}

/// What the Appearance section needs from the app.
pub struct ThemeContext<'a> {
    pub customizer: &'a mut ThemeCustomizer,
    /// Mode of the palette being shown now.
    pub mode: ThemeMode,
}

fn draw_settings_tab(
    ui: &mut egui::Ui,
    tab: SettingsTab,
    settings: &mut SettingsWindow,
    theme: &mut ThemeContext,
    palette: &ThemePalette,
    i18n: &mut I18n,
) -> Option<SettingsAction> {
    match tab {
        SettingsTab::Hotkeys => {
            hotkey_editor::draw(ui, palette, i18n, &mut settings.current_settings.hotkeys);
            None
        }
        SettingsTab::Appearance => theme::draw_appearance_section(
            ui,
            settings,
            theme.customizer,
            theme.mode,
            palette,
            i18n,
        ),
        SettingsTab::Files => sections::draw_files_section(ui, settings, palette, i18n),
        SettingsTab::Interface => sections::draw_interface_section(ui, settings, palette, i18n),
        SettingsTab::Integrations => {
            sections::draw_integrations_section(ui, settings, palette, i18n)
        }
        SettingsTab::General => sections::draw_general_section(ui, settings, palette, i18n),
    }
}

/// The selected section, or, while searching, every section's matching items under its title.
fn draw_settings_content(
    ui: &mut egui::Ui,
    settings: &mut SettingsWindow,
    theme: &mut ThemeContext,
    palette: &ThemePalette,
    i18n: &mut I18n,
    action: &mut Option<SettingsAction>,
) {
    if !search_active(ui.ctx()) {
        let tab = settings.selected_tab;
        settings_page_title(ui, palette, &i18n.tr(tab_label_key(tab)));
        *action = draw_settings_tab(ui, tab, settings, theme, palette, i18n);
        return;
    }

    for (tab, key) in settings_tabs() {
        let tab_action = show_if_matches(ui, |ui| {
            settings_page_title(ui, palette, &i18n.tr(key));
            draw_settings_tab(ui, tab, settings, theme, palette, i18n)
        });
        if tab_action.is_some() {
            *action = tab_action;
        }
    }
    if search_shown_count(ui.ctx()) == 0 {
        ui.add_space(12.0);
        ui.label(
            RichText::new(i18n.tr("settings_search_no_results"))
                .size(body_size(palette))
                .color(palette.text_normal),
        );
    }
}

fn tab_label_key(tab: SettingsTab) -> &'static str {
    settings_tabs()
        .find(|(candidate, _)| *candidate == tab)
        .map_or("settings", |(_, key)| key)
}

/// Title of the section shown at the top of the content.
fn settings_page_title(ui: &mut egui::Ui, palette: &ThemePalette, text: &str) {
    ui.label(
        RichText::new(text)
            .size(body_size(palette) + 6.0)
            .strong()
            .color(palette.text_header_section),
    );
}

/// Spacing and hit targets for the settings window. Text sizes follow the app's own.
fn apply_settings_style(ui: &mut egui::Ui, palette: &ThemePalette) {
    let body = body_size(palette);
    let mut style = (**ui.style()).clone();
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::proportional(body + 2.0),
    );
    for text_style in [egui::TextStyle::Body, egui::TextStyle::Button] {
        style
            .text_styles
            .insert(text_style, egui::FontId::proportional(body));
    }
    style.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::proportional(small_size(palette)),
    );
    style.spacing.item_spacing = egui::vec2(8.0, 4.0);
    style.spacing.interact_size = egui::vec2(40.0, control_height(palette));
    style.spacing.button_padding = egui::vec2(10.0, 4.0);
    style.override_text_valign = Some(egui::Align::Center);
    ui.set_style(style);
}

pub fn draw_settings_window(
    ctx: &egui::Context,
    settings: &mut SettingsWindow,
    theme: &mut ThemeContext,
    i18n: &mut I18n,
    palette: &ThemePalette,
) -> Option<SettingsAction> {
    let mut action = None;

    if !settings.open {
        return None;
    }

    let mut should_close = false;

    // Dark background overlay (modal effect); clicking it dismisses the window
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
    if ctx.input_mut(|i| {
        i.consume_shortcut(&egui::KeyboardShortcut::new(
            egui::Modifiers::COMMAND,
            egui::Key::F,
        ))
    }) {
        ctx.memory_mut(|memory| memory.request_focus(search_field_id()));
    }

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
            begin_search_frame(ctx, &settings.search);

            let size = ui.available_size();
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 14.0;

                ui.allocate_ui_with_layout(
                    egui::vec2(SETTINGS_NAV_WIDTH, size.y),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut settings.search)
                                .id(search_field_id())
                                .hint_text(i18n.tr("settings_search_placeholder"))
                                .font(egui::FontId::proportional(body_size(palette)))
                                .margin(egui::vec2(8.0, 5.0))
                                .desired_width(f32::INFINITY),
                        );
                        ui.spacing_mut().item_spacing.y = 2.0;
                        let searching = !settings.search.trim().is_empty();
                        for (group, tabs) in SETTINGS_GROUPS {
                            nav_group_header(ui, palette, &i18n.tr(group));
                            for &(tab, key) in tabs {
                                let selected = !searching && settings.selected_tab == tab;
                                if nav_item(ui, palette, selected, &i18n.tr(key)).clicked() {
                                    settings.selected_tab = tab;
                                    settings.search.clear();
                                }
                            }
                        }
                    },
                );

                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), size.y),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                // Keep content clear of the scroll bar.
                                ui.set_max_width(ui.available_width() - 14.0);
                                draw_settings_content(
                                    ui,
                                    settings,
                                    theme,
                                    palette,
                                    i18n,
                                    &mut action,
                                );
                            });
                    },
                );
            });

            // These confirmation dialogs live outside the selected-section content so they
            // remain visible if the user changes sections while a destructive action is pending.
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
        settings.search.clear();
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
    fn zoxide_auto_add_defaults_off_and_round_trips() {
        let old: AppSettings = serde_json::from_str(r#"{"language":"en-US"}"#).unwrap();
        assert!(!old.zoxide_auto_add);
        let on = AppSettings {
            zoxide_auto_add: true,
            ..AppSettings::default()
        };
        let back: AppSettings = serde_json::from_str(&serde_json::to_string(&on).unwrap()).unwrap();
        assert!(back.zoxide_auto_add);
    }

    #[test]
    fn escape_closes_settings_only_without_an_open_confirmation() {
        assert!(escape_closes_settings(true, false));
        assert!(!escape_closes_settings(true, true));
        assert!(!escape_closes_settings(false, false));
        assert!(!escape_closes_settings(false, true));
    }

    #[test]
    fn empty_query_matches_everything() {
        assert!(matches_search("", &["Show hidden files"]));
        assert!(matches_search("   ", &[]));
    }

    #[test]
    fn search_is_case_insensitive_substring() {
        assert!(matches_search("HIDDEN", &["Show hidden files/folders"]));
        assert!(matches_search("files", &["Show hidden files/folders"]));
        assert!(!matches_search("sidebar", &["Show hidden files/folders"]));
    }

    #[test]
    fn search_requires_every_term_across_any_text() {
        let texts = ["Date format", "Controls how dates are displayed"];
        assert!(matches_search("date displayed", &texts));
        assert!(matches_search("  format   DATES ", &texts));
        assert!(!matches_search("date sidebar", &texts));
    }

    #[test]
    fn search_matches_non_ascii_text() {
        assert!(matches_search("外観", &["外観"]));
        assert!(matches_search("ÉCOLE", &["école"]));
        assert!(!matches_search("設定", &["外観"]));
    }

    #[test]
    fn search_admits_counts_only_matching_items() {
        let ctx = egui::Context::default();
        begin_search_frame(&ctx, "  hidden ");
        assert!(search_active(&ctx));
        assert!(search_admits(&ctx, &["Show hidden files"]));
        assert!(!search_admits(&ctx, &["Sidebar"]));
        assert!(search_admits(&ctx, &["Other", "a hidden tooltip"]));
        assert_eq!(search_shown_count(&ctx), 2);

        begin_search_frame(&ctx, "   ");
        assert!(!search_active(&ctx));
        assert!(search_admits(&ctx, &["Sidebar"]));
    }
}
