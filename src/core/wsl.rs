//! Installed WSL distributions, for the sidebar's Linux section.
//!
//! Distributions are registered per user under
//! `HKCU\Software\Microsoft\Windows\CurrentVersion\Lxss\{GUID}` with a
//! `DistributionName` value; their files are reachable at
//! `\\wsl.localhost\<name>` (Windows 10 1903+ / 11).

use std::path::PathBuf;
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, RRF_RT_REG_SZ, RegCloseKey, RegEnumKeyExW, RegGetValueW,
    RegOpenKeyExW,
};
use windows::core::{PCWSTR, PWSTR, w};

/// UNC root of a distribution's file system.
pub fn wsl_path(distro: &str) -> PathBuf {
    PathBuf::from(format!(r"\\wsl.localhost\{distro}"))
}

/// Sorted, de-duplicated, non-empty names. Names with path separators are
/// dropped (they cannot be a share name).
pub fn clean_distro_names(names: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut out: Vec<String> = names
        .into_iter()
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty() && !n.contains(['\\', '/']))
        .collect();
    out.sort_by_key(|n| n.to_lowercase());
    out.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    out
}

/// Reads the registered distributions. Every registry call is checked; any
/// failure just yields fewer (or no) entries.
pub fn list_distributions() -> Vec<String> {
    let mut names = Vec::new();
    unsafe {
        let mut root = HKEY::default();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            w!(r"Software\Microsoft\Windows\CurrentVersion\Lxss"),
            None,
            KEY_READ,
            &mut root,
        )
        .is_err()
        {
            return names;
        }

        for index in 0..64u32 {
            let mut buf = [0u16; 128];
            let mut len = buf.len() as u32;
            if RegEnumKeyExW(
                root,
                index,
                Some(PWSTR(buf.as_mut_ptr())),
                &mut len,
                None,
                None,
                None,
                None,
            )
            .is_err()
            {
                break;
            }
            let subkey: Vec<u16> = buf[..len as usize]
                .iter()
                .copied()
                .chain(std::iter::once(0))
                .collect();

            let mut value = [0u16; 256];
            let mut size = (value.len() * 2) as u32;
            if RegGetValueW(
                root,
                PCWSTR(subkey.as_ptr()),
                w!("DistributionName"),
                RRF_RT_REG_SZ,
                None,
                Some(value.as_mut_ptr().cast()),
                Some(&mut size),
            )
            .is_ok()
            {
                let chars = (size as usize / 2).min(value.len());
                let name = String::from_utf16_lossy(&value[..chars]);
                names.push(name.trim_end_matches('\0').to_string());
            }
        }
        let _ = RegCloseKey(root);
    }
    clean_distro_names(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unc_path() {
        assert_eq!(wsl_path("Ubuntu"), PathBuf::from(r"\\wsl.localhost\Ubuntu"));
    }

    #[test]
    fn names_are_cleaned() {
        let names = clean_distro_names(vec![
            "Ubuntu".into(),
            " debian ".into(),
            "".into(),
            "ubuntu".into(),
            "bad\\name".into(),
        ]);
        assert_eq!(names, vec!["debian".to_string(), "Ubuntu".to_string()]);
    }

    #[test]
    fn listing_does_not_panic() {
        // Reads HKCU only; works with or without WSL installed.
        let _ = list_distributions();
    }
}
