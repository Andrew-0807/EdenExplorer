//! Offscreen scene renderer. Scenes draw real app UI into an
//! `egui_kittest::Harness` (wgpu) and write a PNG to `docs/stageshow/`.
//! Run: `STAGESHOW=1 cargo test stageshow -- --nocapture`. Without the env
//! var the scene tests return immediately, so plain `cargo test` needs no GPU.
//! See `docs/stageshow/README.md`.

use crate::core::i18n::I18n;
use crate::gui::folder_icons::{IconStyle, paint_folder_icon, resolve_folder_icon};
use crate::gui::icon_system::{AppIcon, paint_icon};
use crate::gui::theme::{ThemeMode, ThemePalette, apply_theme, get_palette};
use crate::gui::windows::settings::{ThemeContext, draw_settings_window};
use crate::gui::windows::structs::{SettingsTab, SettingsWindow, ThemeCustomizer};
use eframe::egui;
use egui_kittest::Harness;

/// True when `STAGESHOW` is set; scene tests skip otherwise.
fn enabled() -> bool {
    std::env::var_os("STAGESHOW").is_some()
}

/// Renders `scene` once on a `size` canvas at `ppp` and writes
/// `docs/stageshow/<name>.png`. `scene` runs every frame with the
/// root `Ui` (use `ui.ctx()` for windows) and the palette of `mode`.
fn render_scene(
    name: &str,
    size: [f32; 2],
    ppp: f32,
    mode: ThemeMode,
    palette: ThemePalette,
    mut scene: impl FnMut(&mut egui::Ui, &ThemePalette),
) {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(size[0], size[1]))
        .with_pixels_per_point(ppp)
        .wgpu()
        .build_ui(move |ui| {
            let ctx = ui.ctx().clone();
            apply_theme(&ctx, mode, false);
            egui_extras::install_image_loaders(&ctx);
            scene(ui, &palette);
        });
    // Several frames so images, layout and animations settle.
    for _ in 0..4 {
        harness.run();
    }
    let image = harness.render().expect("wgpu render failed");
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/stageshow");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.png"));
    image.save(&path).unwrap();
    println!("stageshow: wrote {}", path.display());
}

/// Renders one settings tab at `text_size`, optionally with a search query.
fn settings_scene(name: &str, tab: SettingsTab, text_size: f32, search: &str, mode: ThemeMode) {
    let mut window = SettingsWindow {
        open: true,
        current_settings: Default::default(),
        show_reset_favorites_confirmation: false,
        show_reset_confirmation: false,
        show_clear_directory_cache_confirmation: false,
        selected_tab: tab,
        search: search.to_string(),
    };
    let mut customizer = ThemeCustomizer::default();
    let mut i18n = I18n::new("en-US");
    let mut palette = get_palette(mode);
    palette.text_size = text_size;
    render_scene(
        name,
        [1000.0, 720.0],
        1.0,
        mode,
        palette,
        move |ui, palette| {
            draw_settings_window(
                ui.ctx(),
                &mut window,
                &mut ThemeContext {
                    customizer: &mut customizer,
                    mode,
                },
                &mut i18n,
                palette,
            );
        },
    );
}

#[test]
fn stageshow_settings() {
    if !enabled() {
        return;
    }
    let tabs = [
        ("general", SettingsTab::General),
        ("files", SettingsTab::Files),
        ("interface", SettingsTab::Interface),
        ("appearance", SettingsTab::Appearance),
        ("hotkeys", SettingsTab::Hotkeys),
        ("integrations", SettingsTab::Integrations),
    ];
    for (name, tab) in tabs {
        settings_scene(&format!("settings_{name}"), tab, 12.0, "", ThemeMode::Dark);
    }
    settings_scene(
        "settings_general_18",
        SettingsTab::General,
        18.0,
        "",
        ThemeMode::Dark,
    );
    settings_scene(
        "settings_search_color",
        SettingsTab::General,
        12.0,
        "color",
        ThemeMode::Dark,
    );
    settings_scene(
        "settings_appearance_light",
        SettingsTab::Appearance,
        12.0,
        "",
        ThemeMode::Light,
    );
}

#[test]
fn stageshow_item_list() {
    if !enabled() {
        return;
    }
    let mode = ThemeMode::Dark;
    let mut palette = get_palette(mode);
    palette.icon_style = IconStyle::Flat;
    let rows: [(&str, &str, bool); 6] = [
        ("Documents", "Folder", true),
        ("Downloads", "Folder", true),
        ("Projects", "Folder", true),
        ("notes.txt", "Text file", false),
        ("report.pdf", "PDF file", false),
        ("budget.xlsx", "Spreadsheet", false),
    ];
    render_scene(
        "item_list",
        [640.0, 300.0],
        1.0,
        mode,
        palette,
        move |ui, palette| {
            let full = ui.ctx().content_rect();
            ui.painter()
                .rect_filled(full, 0.0, palette.application_bg_color);
            egui::Frame::new().show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.set_min_size(full.size());
                for (i, (name, kind, is_dir)) in rows.iter().enumerate() {
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 36.0),
                        egui::Sense::click(),
                    );
                    let fill = if i == 1 {
                        palette.row_selected_bg
                    } else {
                        palette.row_bg
                    };
                    ui.painter().rect_filled(rect, 0.0, fill);
                    let icon_rect = egui::Rect::from_center_size(
                        rect.left_center() + egui::vec2(24.0, 0.0),
                        egui::Vec2::splat(22.0),
                    );
                    let folder = resolve_folder_icon(palette, Some(name), *is_dir);
                    if !paint_folder_icon(ui, icon_rect, 22.0, folder, false) {
                        paint_icon(
                            ui,
                            icon_rect,
                            AppIcon::File.regular(),
                            22.0,
                            palette.icon_color,
                        );
                    }
                    let font = egui::FontId::proportional(palette.text_size + 2.0);
                    ui.painter().text(
                        rect.left_center() + egui::vec2(52.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        *name,
                        font.clone(),
                        palette.text_normal,
                    );
                    ui.painter().text(
                        rect.left_center() + egui::vec2(380.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        *kind,
                        font,
                        palette.text_normal.linear_multiply(0.6),
                    );
                }
            });
        },
    );
}
