//! Windows accent color and app light/dark mode.
//!
//! Read on startup and again when the window receives `WM_SETTINGCHANGE` or
//! `WM_DWMCOLORIZATIONCOLORCHANGED` (the window procedure sets a flag and
//! wakes the UI), so nothing is polled.

use eframe::egui::Color32;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::Graphics::Dwm::DwmGetColorizationColor;
use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
use windows::core::{BOOL, PCWSTR, w};

static SYSTEM_THEME_DIRTY: AtomicBool = AtomicBool::new(true);

/// Called from the window procedure when Windows reports a theme change.
pub fn mark_system_theme_dirty() {
    SYSTEM_THEME_DIRTY.store(true, Ordering::Release);
}

/// True once after each change (and once at startup).
pub fn consume_system_theme_dirty() -> bool {
    SYSTEM_THEME_DIRTY.swap(false, Ordering::AcqRel)
}

/// `DWM\AccentColor` is stored as 0xAABBGGRR.
pub fn decode_abgr(value: u32) -> Color32 {
    let r = (value & 0xFF) as u8;
    let g = ((value >> 8) & 0xFF) as u8;
    let b = ((value >> 16) & 0xFF) as u8;
    Color32::from_rgb(r, g, b)
}

/// `DwmGetColorizationColor` returns 0xAARRGGBB.
pub fn decode_argb(value: u32) -> Color32 {
    let b = (value & 0xFF) as u8;
    let g = ((value >> 8) & 0xFF) as u8;
    let r = ((value >> 16) & 0xFF) as u8;
    Color32::from_rgb(r, g, b)
}

fn read_dword(subkey: PCWSTR, value: PCWSTR) -> Option<u32> {
    let mut data = 0u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            subkey,
            value,
            RRF_RT_REG_DWORD,
            None,
            Some((&mut data as *mut u32).cast()),
            Some(&mut size),
        )
    };
    status.is_ok().then_some(data)
}

/// The user's accent color, or `None` if it cannot be read.
pub fn accent_color() -> Option<Color32> {
    if let Some(value) = read_dword(w!(r"Software\Microsoft\Windows\DWM"), w!("AccentColor")) {
        return Some(decode_abgr(value));
    }
    let mut color = 0u32;
    let mut opaque = BOOL(0);
    unsafe { DwmGetColorizationColor(&mut color, &mut opaque) }
        .ok()
        .map(|_| decode_argb(color))
}

/// `Some(true)` when apps should use the light theme.
pub fn apps_use_light_theme() -> Option<bool> {
    read_dword(
        w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"),
        w!("AppsUseLightTheme"),
    )
    .map(|v| v != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_the_registry_accent() {
        // The value on the machine this was written on: 0xFF8C8683.
        assert_eq!(
            decode_abgr(0xFF8C_8683),
            Color32::from_rgb(0x83, 0x86, 0x8C)
        );
        assert_eq!(
            decode_abgr(0xFFD7_7800),
            Color32::from_rgb(0x00, 0x78, 0xD7)
        );
    }

    #[test]
    fn decodes_the_dwm_colorization() {
        assert_eq!(
            decode_argb(0xC483_868C),
            Color32::from_rgb(0x83, 0x86, 0x8C)
        );
    }

    #[test]
    fn dirty_flag_is_consumed_once() {
        mark_system_theme_dirty();
        assert!(consume_system_theme_dirty());
        assert!(!consume_system_theme_dirty());
    }

    #[test]
    fn reading_does_not_panic() {
        let _ = accent_color();
        let _ = apps_use_light_theme();
    }
}
