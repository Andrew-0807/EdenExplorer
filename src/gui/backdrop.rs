//! Window backdrop (Mica / Acrylic) decisions. The Win32 calls live in
//! `windows/windowsoverrides.rs`; everything here is pure and tested.
//!
//! How it fits together:
//! 1. At startup `main.rs` asks [`wants_transparent_surface`] whether any saved
//!    palette uses a backdrop. Only then is the window created transparent and
//!    wgpu told to present through DirectComposition (`DxgiFromVisual`), the
//!    only DX12 swapchain kind with per-pixel alpha. Users without a backdrop
//!    keep the exact opaque swapchain they had before.
//! 2. When the theme is applied, `apply_backdrop` sets
//!    `DWMWA_SYSTEMBACKDROP_TYPE`. [`backdrop_effective`] combines its result
//!    with the surface state; if anything is missing (Windows 10, no DX12,
//!    surface created opaque) the window keeps its opaque background.
//! 3. DWM composes Mica/Acrylic itself: no extra repaints are needed.

use crate::gui::theme::ThemePalette;
use eframe::egui::{self, Color32};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Backdrop {
    #[default]
    None,
    Mica,
    MicaAlt,
    Acrylic,
}

impl Backdrop {
    pub const ALL: [Backdrop; 4] = [
        Backdrop::None,
        Backdrop::Mica,
        Backdrop::MicaAlt,
        Backdrop::Acrylic,
    ];

    /// `DWM_SYSTEMBACKDROP_TYPE` value for `DWMWA_SYSTEMBACKDROP_TYPE` (38).
    pub fn dwm_value(self) -> u32 {
        match self {
            Backdrop::None => 1,    // DWMSBT_NONE
            Backdrop::Mica => 2,    // DWMSBT_MAINWINDOW
            Backdrop::Acrylic => 3, // DWMSBT_TRANSIENTWINDOW
            Backdrop::MicaAlt => 4, // DWMSBT_TABBEDWINDOW
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Backdrop::None => "theme_backdrop_none",
            Backdrop::Mica => "theme_backdrop_mica",
            Backdrop::MicaAlt => "theme_backdrop_mica_alt",
            Backdrop::Acrylic => "theme_backdrop_acrylic",
        }
    }
}

static TRANSPARENT_SURFACE: AtomicBool = AtomicBool::new(false);

/// Recorded once at startup: whether the window was created transparent.
pub fn set_transparent_surface(enabled: bool) {
    TRANSPARENT_SURFACE.store(enabled, Ordering::Release);
}

pub fn transparent_surface() -> bool {
    TRANSPARENT_SURFACE.load(Ordering::Acquire)
}

/// Whether the window must be created with a transparent surface, given the
/// saved palettes.
pub fn wants_transparent_surface(palettes: &[&ThemePalette]) -> bool {
    palettes.iter().any(|p| p.backdrop != Backdrop::None)
}

/// A backdrop really shows only when it was requested, the surface can
/// carry alpha, the renderer is DX12 (Vulkan/GL surfaces on Windows are
/// opaque) and DWM accepted the attribute.
pub fn backdrop_effective(
    requested: Backdrop,
    transparent_surface: bool,
    dx12: bool,
    dwm_accepted: bool,
) -> bool {
    requested != Backdrop::None && transparent_surface && dx12 && dwm_accepted
}

/// The backdrop is configured but cannot show until EdenExplorer restarts
/// with a transparent surface (shown as a hint in the theme editor).
pub fn needs_restart(requested: Backdrop, transparent_surface: bool) -> bool {
    requested != Backdrop::None && !transparent_surface
}

/// Background color of the window: opaque normally, `application_bg_color`
/// at `backdrop_tint_opacity` over an active backdrop.
pub fn panel_fill(palette: &ThemePalette, backdrop_active: bool) -> Color32 {
    let bg = palette.application_bg_color;
    let fill = if backdrop_active {
        let alpha = (palette.backdrop_tint_opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
        Color32::from_rgba_unmultiplied(bg.r(), bg.g(), bg.b(), alpha)
    } else {
        Color32::from_rgb(bg.r(), bg.g(), bg.b())
    };
    if palette.bg_gradient {
        // The gradient is painted separately; see `gradient_colors`.
        Color32::TRANSPARENT
    } else {
        fill
    }
}

/// Top and bottom colors of the background gradient, with the same alpha
/// the flat fill would have.
pub fn gradient_colors(palette: &ThemePalette, backdrop_active: bool) -> (Color32, Color32) {
    let mut flat = palette.clone();
    flat.bg_gradient = false;
    let top = panel_fill(&flat, backdrop_active);
    let end = palette.bg_gradient_end;
    let bottom = Color32::from_rgba_unmultiplied(end.r(), end.g(), end.b(), top.a());
    (top, bottom)
}

/// Four-vertex vertical gradient covering `rect`.
pub fn gradient_mesh(rect: egui::Rect, top: Color32, bottom: Color32) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    mesh
}

/// Clear color handed to eframe: fully transparent when the surface is
/// transparent (the panels decide the tint), else opaque.
pub fn clear_color(palette: &ThemePalette, transparent_surface: bool) -> [f32; 4] {
    if transparent_surface {
        [0.0, 0.0, 0.0, 0.0]
    } else {
        let bg = palette.application_bg_color;
        Color32::from_rgb(bg.r(), bg.g(), bg.b()).to_normalized_gamma_f32()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::theme::{ThemeMode, get_default_palette};

    #[test]
    fn dwm_values_match_the_sdk() {
        assert_eq!(Backdrop::None.dwm_value(), 1);
        assert_eq!(Backdrop::Mica.dwm_value(), 2);
        assert_eq!(Backdrop::Acrylic.dwm_value(), 3);
        assert_eq!(Backdrop::MicaAlt.dwm_value(), 4);
    }

    #[test]
    fn backdrop_needs_every_ingredient() {
        assert!(backdrop_effective(Backdrop::Mica, true, true, true));
        assert!(!backdrop_effective(Backdrop::None, true, true, true));
        assert!(!backdrop_effective(Backdrop::Mica, false, true, true));
        assert!(!backdrop_effective(Backdrop::Mica, true, false, true));
        // Windows 10 rejects DWMWA_SYSTEMBACKDROP_TYPE.
        assert!(!backdrop_effective(Backdrop::Acrylic, true, true, false));
    }

    #[test]
    fn restart_hint_only_when_the_surface_is_opaque() {
        assert!(needs_restart(Backdrop::Mica, false));
        assert!(!needs_restart(Backdrop::Mica, true));
        assert!(!needs_restart(Backdrop::None, false));
    }

    #[test]
    fn default_themes_keep_an_opaque_window() {
        let dark = get_default_palette(ThemeMode::Dark);
        let light = get_default_palette(ThemeMode::Light);
        assert!(!wants_transparent_surface(&[&dark, &light]));
        assert_eq!(panel_fill(&dark, false).a(), 255);
        assert_eq!(clear_color(&dark, false)[3], 1.0);

        let mut mica = dark.clone();
        mica.backdrop = Backdrop::Mica;
        assert!(wants_transparent_surface(&[&light, &mica]));
    }

    #[test]
    fn tint_opacity_controls_alpha_only_when_active() {
        let mut p = get_default_palette(ThemeMode::Dark);
        p.backdrop = Backdrop::Mica;
        p.backdrop_tint_opacity = 0.5;
        assert_eq!(panel_fill(&p, true).a(), 128);
        assert_eq!(panel_fill(&p, false).a(), 255);
        p.backdrop_tint_opacity = 7.0;
        assert_eq!(panel_fill(&p, true).a(), 255);
        assert_eq!(clear_color(&p, true), [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn gradient_replaces_the_flat_fill() {
        let mut p = get_default_palette(ThemeMode::Dark);
        p.bg_gradient = true;
        p.backdrop_tint_opacity = 0.5;
        assert_eq!(panel_fill(&p, false), Color32::TRANSPARENT);
        let (top, bottom) = gradient_colors(&p, true);
        assert_eq!(top.a(), 128);
        assert_eq!(bottom.a(), 128);
        let (top, _) = gradient_colors(&p, false);
        assert_eq!(top, p.application_bg_color);

        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(10.0, 20.0));
        let mesh = gradient_mesh(rect, top, Color32::RED);
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices.len(), 6);
        assert_eq!(mesh.vertices[3].color, Color32::RED);
    }
}
