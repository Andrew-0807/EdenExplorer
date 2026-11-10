//! Built-in theme presets and theme file import/export.

use crate::gui::backdrop::Backdrop;
use crate::gui::folder_icons::IconStyle;
use crate::gui::selection::SelectionStyle;
use crate::gui::theme::{
    THEME_VERSION, ThemeMode, ThemePalette, get_default_palette, palette_from_json,
    regenerate_base_derived_colors,
};
use eframe::egui::Color32;
use serde_json::Value;

/// `format` tag of an exported theme file.
pub const THEME_FILE_FORMAT: &str = "eden-theme";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemePreset {
    Eden,
    Windows11,
    Nord,
    Dracula,
    HighContrast,
}

impl ThemePreset {
    pub const ALL: [ThemePreset; 5] = [
        ThemePreset::Eden,
        ThemePreset::Windows11,
        ThemePreset::Nord,
        ThemePreset::Dracula,
        ThemePreset::HighContrast,
    ];

    pub fn label_key(self) -> &'static str {
        match self {
            ThemePreset::Eden => "theme_preset_eden",
            ThemePreset::Windows11 => "theme_preset_windows11",
            ThemePreset::Nord => "theme_preset_nord",
            ThemePreset::Dracula => "theme_preset_dracula",
            ThemePreset::HighContrast => "theme_preset_high_contrast",
        }
    }
}

/// The handful of colors a preset defines; everything else is derived from
/// them or kept from the mode's defaults.
struct PresetColors {
    primary: Color32,
    bg: Color32,
    row_bg: Color32,
    row_selected_bg: Color32,
    text: Color32,
    header: Color32,
    widget_bg: Color32,
    faint_bg: Color32,
    widget_text: Color32,
    widget_text_hover: Color32,
    icon: Color32,
}

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

fn preset_colors(preset: ThemePreset, mode: ThemeMode) -> Option<PresetColors> {
    let dark = mode == ThemeMode::Dark;
    let colors = match (preset, dark) {
        (ThemePreset::Eden, _) => return None,
        // Windows 11 Fluent neutrals and default accent.
        (ThemePreset::Windows11, true) => PresetColors {
            primary: rgb(96, 205, 255),
            bg: rgb(32, 32, 32),
            row_bg: rgb(45, 45, 45),
            row_selected_bg: rgb(61, 61, 61),
            text: rgb(206, 206, 206),
            header: Color32::WHITE,
            widget_bg: rgb(45, 45, 45),
            faint_bg: rgb(39, 39, 39),
            widget_text: rgb(230, 230, 230),
            widget_text_hover: Color32::WHITE,
            icon: rgb(222, 222, 222),
        },
        (ThemePreset::Windows11, false) => PresetColors {
            primary: rgb(0, 95, 184),
            bg: rgb(243, 243, 243),
            row_bg: rgb(251, 251, 251),
            row_selected_bg: rgb(222, 232, 244),
            text: rgb(58, 58, 58),
            header: rgb(26, 26, 26),
            widget_bg: Color32::WHITE,
            faint_bg: rgb(249, 249, 249),
            widget_text: rgb(27, 27, 27),
            widget_text_hover: Color32::BLACK,
            icon: rgb(40, 40, 40),
        },
        (ThemePreset::Nord, true) => PresetColors {
            primary: rgb(136, 192, 208),
            bg: rgb(46, 52, 64),
            row_bg: rgb(59, 66, 82),
            row_selected_bg: rgb(67, 76, 94),
            text: rgb(216, 222, 233),
            header: rgb(236, 239, 244),
            widget_bg: rgb(59, 66, 82),
            faint_bg: rgb(52, 58, 71),
            widget_text: rgb(229, 233, 240),
            widget_text_hover: rgb(236, 239, 244),
            icon: rgb(216, 222, 233),
        },
        (ThemePreset::Nord, false) => PresetColors {
            primary: rgb(94, 129, 172),
            bg: rgb(236, 239, 244),
            row_bg: rgb(229, 233, 240),
            row_selected_bg: rgb(216, 222, 233),
            text: rgb(46, 52, 64),
            header: rgb(36, 41, 51),
            widget_bg: rgb(229, 233, 240),
            faint_bg: rgb(232, 235, 241),
            widget_text: rgb(46, 52, 64),
            widget_text_hover: rgb(36, 41, 51),
            icon: rgb(59, 66, 82),
        },
        (ThemePreset::Dracula, true) => PresetColors {
            primary: rgb(189, 147, 249),
            bg: rgb(40, 42, 54),
            row_bg: rgb(68, 71, 90),
            row_selected_bg: rgb(68, 71, 90),
            text: rgb(248, 248, 242),
            header: Color32::WHITE,
            widget_bg: rgb(68, 71, 90),
            faint_bg: rgb(33, 34, 44),
            widget_text: rgb(248, 248, 242),
            widget_text_hover: Color32::WHITE,
            icon: rgb(248, 248, 242),
        },
        // "Alucard", the light Dracula variant.
        (ThemePreset::Dracula, false) => PresetColors {
            primary: rgb(100, 74, 201),
            bg: rgb(255, 251, 235),
            row_bg: rgb(245, 241, 225),
            row_selected_bg: rgb(229, 224, 206),
            text: rgb(31, 31, 31),
            header: Color32::BLACK,
            widget_bg: rgb(245, 241, 225),
            faint_bg: rgb(250, 246, 230),
            widget_text: rgb(31, 31, 31),
            widget_text_hover: Color32::BLACK,
            icon: rgb(60, 60, 60),
        },
        (ThemePreset::HighContrast, true) => PresetColors {
            primary: rgb(255, 255, 0),
            bg: Color32::BLACK,
            row_bg: rgb(16, 16, 16),
            row_selected_bg: Color32::BLACK,
            text: Color32::WHITE,
            header: Color32::WHITE,
            widget_bg: Color32::BLACK,
            faint_bg: rgb(10, 10, 10),
            widget_text: Color32::WHITE,
            widget_text_hover: rgb(255, 255, 0),
            icon: Color32::WHITE,
        },
        (ThemePreset::HighContrast, false) => PresetColors {
            primary: rgb(0, 0, 160),
            bg: Color32::WHITE,
            row_bg: Color32::WHITE,
            row_selected_bg: rgb(230, 230, 255),
            text: Color32::BLACK,
            header: Color32::BLACK,
            widget_bg: Color32::WHITE,
            faint_bg: rgb(245, 245, 245),
            widget_text: Color32::BLACK,
            widget_text_hover: rgb(0, 0, 160),
            icon: Color32::BLACK,
        },
    };
    Some(colors)
}

/// Palette for `preset` in `mode`. Sizes and the font are taken from `base`
/// (the palette being edited), so choosing a preset never changes layout.
pub fn preset_palette(preset: ThemePreset, mode: ThemeMode, base: &ThemePalette) -> ThemePalette {
    let mut palette = get_default_palette(mode);
    preset_apply_extras(preset, mode, &mut palette);
    if let Some(c) = preset_colors(preset, mode) {
        palette.primary = c.primary;
        regenerate_base_derived_colors(&mut palette, mode == ThemeMode::Dark);
        palette.application_bg_color = c.bg;
        palette.row_bg = c.row_bg;
        palette.row_selected_bg = c.row_selected_bg;
        palette.text_normal = c.text;
        palette.text_header_section = c.header;
        palette.item_viewer_row_text_selected = c.header;
        palette.tooltip_text_color = c.text;
        palette.tab_close_normal = c.text;
        palette.widget_bg = c.widget_bg;
        palette.faint_bg = c.faint_bg;
        palette.widget_text = c.widget_text;
        palette.widget_text_hover = c.widget_text_hover;
        palette.icon_color = c.icon;
        palette.drive_usage_text = c.text;
    }
    keep_layout(&mut palette, base);
    palette
}

/// Non-color settings a preset turns on (backdrop, selection style, ...).
/// Later features hook in here.
fn preset_apply_extras(preset: ThemePreset, mode: ThemeMode, palette: &mut ThemePalette) {
    if preset == ThemePreset::Windows11 {
        // Mica with a light tint and no separator lines, like Explorer with
        // ExplorerBlurMica (dark tint black at 70/255, light white at 200/255).
        palette.backdrop = Backdrop::Mica;
        palette.backdrop_tint_opacity = match mode {
            ThemeMode::Dark => 0.45,
            ThemeMode::Light => 0.75,
        };
        palette.show_separators = false;
        // Explorer draws the selected row as a thin light outline.
        palette.selection_style = SelectionStyle::Outline;
        palette.selection_radius = 2;
        // Shell icons, so icon packs (e.g. Windhawk icon themes) show as in Explorer.
        palette.icon_style = IconStyle::Native;
        palette.selection_stroke = match mode {
            ThemeMode::Dark => rgb(220, 220, 220),
            ThemeMode::Light => rgb(40, 40, 40),
        };
    }
}

fn keep_layout(palette: &mut ThemePalette, base: &ThemePalette) {
    palette.text_size = base.text_size;
    palette.tooltip_text_size = base.tooltip_text_size;
    palette.sidebar_icon_size = base.sidebar_icon_size;
    palette.tab_icon_size = base.tab_icon_size;
    palette.font_name = base.font_name.clone();
    palette.row_height = base.row_height;
    palette.sidebar_item_spacing_y = base.sidebar_item_spacing_y;
}

/// Row and icon size presets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Density {
    Compact,
    Normal,
    Touch,
}

impl Density {
    pub const ALL: [Density; 3] = [Density::Compact, Density::Normal, Density::Touch];

    pub fn label_key(self) -> &'static str {
        match self {
            Density::Compact => "theme_density_compact",
            Density::Normal => "theme_density_normal",
            Density::Touch => "theme_density_touch",
        }
    }
}

/// Sets row height, sidebar spacing and icon sizes for `density`; colors,
/// fonts and text sizes are left alone.
pub fn apply_density(palette: &mut ThemePalette, density: Density) {
    let (row, spacing, sidebar_icon, tab_icon) = match density {
        Density::Compact => (18.0, 0.3, 16.0, 14.0),
        Density::Normal => (22.0, 0.5, 18.0, 16.0),
        Density::Touch => (30.0, 1.6, 22.0, 18.0),
    };
    palette.row_height = row;
    palette.sidebar_item_spacing_y = spacing;
    palette.sidebar_icon_size = sidebar_icon;
    palette.tab_icon_size = tab_icon;
}

/// Serializes one palette as an exported theme file.
pub fn export_theme_json(mode: ThemeMode, palette: &ThemePalette) -> Option<String> {
    let mut map = serde_json::Map::new();
    map.insert("format".into(), Value::from(THEME_FILE_FORMAT));
    map.insert("version".into(), Value::from(THEME_VERSION));
    map.insert(
        "mode".into(),
        Value::from(match mode {
            ThemeMode::Dark => "dark",
            ThemeMode::Light => "light",
        }),
    );
    map.insert("palette".into(), serde_json::to_value(palette).ok()?);
    serde_json::to_string_pretty(&Value::Object(map)).ok()
}

#[derive(Debug, PartialEq, Eq)]
pub enum ThemeImportError {
    /// Not JSON at all.
    NotJson,
    /// JSON, but neither an exported theme nor a palette.
    NotATheme,
}

/// Fields every palette has; a bare object needs at least one of them to be
/// accepted as a v1.3.0 export.
const PALETTE_MARKER_FIELDS: [&str; 4] = [
    "primary",
    "application_bg_color",
    "text_size",
    "text_normal",
];

/// Reads an exported theme into a palette for `mode`. Accepts the current
/// format, the bare palette JSON written by v1.3.0, and partial files
/// (missing fields keep `mode`'s defaults).
pub fn import_theme_json(text: &str, mode: ThemeMode) -> Result<ThemePalette, ThemeImportError> {
    let value: Value = serde_json::from_str(text).map_err(|_| ThemeImportError::NotJson)?;
    let Value::Object(mut map) = value else {
        return Err(ThemeImportError::NotATheme);
    };

    let palette = match map.get("format").and_then(Value::as_str) {
        Some(THEME_FILE_FORMAT) => map.remove("palette").unwrap_or(Value::Null),
        Some(_) => return Err(ThemeImportError::NotATheme),
        None if PALETTE_MARKER_FIELDS.iter().any(|k| map.contains_key(*k)) => Value::Object(map),
        None => return Err(ThemeImportError::NotATheme),
    };
    if !palette.is_object() {
        return Err(ThemeImportError::NotATheme);
    }
    Ok(palette_from_json(mode, palette))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::legacy::ThemeSettingsSnapshotV8;
    use crate::gui::theme::active_tab_text_color;
    use crate::gui::theme::tests::assert_contrast;

    #[test]
    fn every_preset_is_readable() {
        for preset in ThemePreset::ALL {
            for mode in [ThemeMode::Dark, ThemeMode::Light] {
                let base = get_default_palette(mode);
                let p = preset_palette(preset, mode, &base);
                let name = format!("{preset:?}/{mode:?}");
                assert_contrast(
                    &format!("{name} body"),
                    p.text_normal,
                    p.application_bg_color,
                );
                assert_contrast(&format!("{name} body on row"), p.text_normal, p.row_bg);
                assert_contrast(
                    &format!("{name} header"),
                    p.text_header_section,
                    p.application_bg_color,
                );
                assert_contrast(
                    &format!("{name} rename field"),
                    p.item_viewer_row_text_selected,
                    p.row_selected_bg,
                );
                assert_contrast(
                    &format!("{name} active tab"),
                    active_tab_text_color(&p, mode == ThemeMode::Dark),
                    p.primary_active,
                );
                assert_contrast(&format!("{name} controls"), p.widget_text, p.widget_bg);
            }
        }
    }

    #[test]
    fn presets_keep_the_layout_of_the_edited_palette() {
        let mut base = get_default_palette(ThemeMode::Dark);
        base.text_size = 15.0;
        base.row_height = 30.0;
        base.font_name = "Consolas".into();
        let p = preset_palette(ThemePreset::Nord, ThemeMode::Dark, &base);
        assert_eq!(p.text_size, 15.0);
        assert_eq!(p.row_height, 30.0);
        assert_eq!(p.font_name, "Consolas");
        assert_eq!(p.primary, rgb(136, 192, 208));
    }

    #[test]
    fn windows11_preset_uses_mica_without_separators() {
        let base = get_default_palette(ThemeMode::Dark);
        let p = preset_palette(ThemePreset::Windows11, ThemeMode::Dark, &base);
        assert_eq!(p.backdrop, Backdrop::Mica);
        assert!(!p.show_separators);
        let p = preset_palette(ThemePreset::Nord, ThemeMode::Dark, &base);
        assert_eq!(p.backdrop, Backdrop::None);
    }

    #[test]
    fn density_presets() {
        let mut p = get_default_palette(ThemeMode::Dark);
        apply_density(&mut p, Density::Normal);
        let d = get_default_palette(ThemeMode::Dark);
        assert_eq!(
            (
                p.row_height,
                p.sidebar_icon_size,
                p.tab_icon_size,
                p.sidebar_item_spacing_y
            ),
            (
                d.row_height,
                d.sidebar_icon_size,
                d.tab_icon_size,
                d.sidebar_item_spacing_y
            )
        );
        apply_density(&mut p, Density::Compact);
        assert!(p.row_height < d.row_height);
        apply_density(&mut p, Density::Touch);
        assert!(p.row_height >= 28.0);
        assert_eq!(p.text_size, d.text_size);
    }

    #[test]
    fn eden_preset_is_the_default() {
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            let base = get_default_palette(mode);
            let p = preset_palette(ThemePreset::Eden, mode, &base);
            assert_eq!(
                serde_json::to_value(&p).unwrap(),
                serde_json::to_value(&base).unwrap()
            );
        }
    }

    #[test]
    fn export_then_import_round_trips() {
        let mut p = get_default_palette(ThemeMode::Light);
        p.primary = rgb(1, 2, 3);
        p.text_size = 14.0;
        let json = export_theme_json(ThemeMode::Light, &p).unwrap();
        assert!(json.contains("\"format\": \"eden-theme\""));
        let back = import_theme_json(&json, ThemeMode::Light).unwrap();
        assert_eq!(
            serde_json::to_value(&back).unwrap(),
            serde_json::to_value(&p).unwrap()
        );
    }

    #[test]
    fn imports_a_v1_3_0_export() {
        // v1.3.0 exported the bare palette with serde_json::to_string_pretty.
        let snapshot: ThemeSettingsSnapshotV8 =
            postcard::from_bytes(include_bytes!("../core/testdata/theme_v1_3_0.bin")).unwrap();
        let old_export = serde_json::to_string_pretty(&snapshot.dark).unwrap();
        let p = import_theme_json(&old_export, ThemeMode::Dark).unwrap();
        assert_eq!(p.primary, rgb(1, 2, 3));
        assert_eq!(p.font_name, "Cascadia Code");
        // New fields get the dark defaults.
        assert_eq!(p.widget_bg, get_default_palette(ThemeMode::Dark).widget_bg);
    }

    #[test]
    fn imports_a_partial_file() {
        let json = r#"{"format":"eden-theme","version":9,"palette":{"primary":[9,9,9,255]}}"#;
        let p = import_theme_json(json, ThemeMode::Dark).unwrap();
        assert_eq!(p.primary, rgb(9, 9, 9));
        assert_eq!(
            p.row_height,
            get_default_palette(ThemeMode::Dark).row_height
        );
    }

    #[test]
    fn rejects_files_that_are_not_themes() {
        assert_eq!(
            import_theme_json("not json", ThemeMode::Dark).unwrap_err(),
            ThemeImportError::NotJson
        );
        assert_eq!(
            import_theme_json("[1,2,3]", ThemeMode::Dark).unwrap_err(),
            ThemeImportError::NotATheme
        );
        assert_eq!(
            import_theme_json(r#"{"name":"x"}"#, ThemeMode::Dark).unwrap_err(),
            ThemeImportError::NotATheme
        );
        assert_eq!(
            import_theme_json(r#"{"format":"eden-settings"}"#, ThemeMode::Dark).unwrap_err(),
            ThemeImportError::NotATheme
        );
        assert_eq!(
            import_theme_json(r#"{"format":"eden-theme","palette":3}"#, ThemeMode::Dark)
                .unwrap_err(),
            ThemeImportError::NotATheme
        );
    }
}
