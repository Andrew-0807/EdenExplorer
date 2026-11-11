//! VS Code icon themes for files and folders.
//!
//! Catppuccin Mocha and Latte are bundled (packed into one JSON each by
//! `scripts/pack-icon-theme.py`). Other packs load from
//! `%LOCALAPPDATA%\Axis\icon-themes\<folder>`, which may hold an unzipped
//! `.vsix` (`extension\package.json`), a folder with `package.json`, or a bare
//! `theme.json`. Only SVG icons are supported, and packs are read once, at
//! first use.

use crate::core::persist;
use eframe::egui::{self, Color32, Image, Rect, Vec2, load::Bytes};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, OnceLock};

const BUNDLED: [(&str, &str, &str); 2] = [
    (
        "catppuccin-mocha",
        "Catppuccin Mocha",
        include_str!("../assets/icon-packs/catppuccin/mocha.json"),
    ),
    (
        "catppuccin-latte",
        "Catppuccin Latte",
        include_str!("../assets/icon-packs/catppuccin/latte.json"),
    ),
];

/// Folder under the data dir that holds user icon packs.
const USER_DIR: &str = "icon-themes";

static PACKS: LazyLock<Vec<IconPack>> = LazyLock::new(|| {
    let mut packs: Vec<IconPack> = BUNDLED
        .iter()
        .filter_map(|(id, label, json)| {
            let manifest = serde_json::from_str(json).ok()?;
            Some(IconPack::new(id, label, manifest, None))
        })
        .collect();
    if let Some(dir) = persist::data_dir().map(|d| d.join(USER_DIR)) {
        // Create it so users can find where packs go.
        let _ = std::fs::create_dir_all(&dir);
        packs.extend(load_user_packs(&dir));
    }
    packs
});

/// The subset of a VS Code icon theme manifest Axis uses.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Manifest {
    icon_definitions: HashMap<String, Definition>,
    file: Option<String>,
    folder: Option<String>,
    file_extensions: HashMap<String, String>,
    file_names: HashMap<String, String>,
    folder_names: HashMap<String, String>,
    /// iconPath -> SVG text, only in bundled packs.
    svgs: HashMap<String, String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Definition {
    icon_path: Option<String>,
}

/// One icon of a pack. User pack files are read on first paint.
#[derive(Debug)]
pub struct PackSvg {
    uri: String,
    path: Option<PathBuf>,
    bytes: OnceLock<Option<Arc<[u8]>>>,
}

impl PartialEq for PackSvg {
    fn eq(&self, other: &Self) -> bool {
        self.uri == other.uri
    }
}

impl PackSvg {
    fn bytes(&self) -> Option<Arc<[u8]>> {
        self.bytes
            .get_or_init(|| std::fs::read(self.path.as_ref()?).ok().map(Arc::from))
            .clone()
    }
}

pub struct IconPack {
    pub id: String,
    pub label: String,
    /// By icon definition id.
    icons: HashMap<String, PackSvg>,
    file: Option<String>,
    folder: Option<String>,
    // Keys are lowercase: VS Code matches names case-insensitively.
    file_extensions: HashMap<String, String>,
    file_names: HashMap<String, String>,
    folder_names: HashMap<String, String>,
}

impl IconPack {
    /// `base` is the manifest's folder for user packs; bundled packs carry
    /// their SVGs inline.
    fn new(id: &str, label: &str, mut manifest: Manifest, base: Option<&Path>) -> Self {
        let icons = manifest
            .icon_definitions
            .into_iter()
            .filter_map(|(def_id, def)| {
                let icon_path = def.icon_path.filter(|p| p.ends_with(".svg"))?;
                let uri = format!("bytes://icon-pack/{id}/{def_id}.svg");
                let svg = match base {
                    Some(base) => PackSvg {
                        uri,
                        path: Some(base.join(&icon_path)),
                        bytes: OnceLock::new(),
                    },
                    None => {
                        let text = manifest.svgs.remove(&icon_path)?;
                        PackSvg {
                            uri,
                            path: None,
                            bytes: OnceLock::from(Some(Arc::from(text.into_bytes()))),
                        }
                    }
                };
                Some((def_id, svg))
            })
            .collect();
        let lower = |map: HashMap<String, String>| {
            map.into_iter()
                .map(|(k, v)| (k.to_lowercase(), v))
                .collect()
        };
        Self {
            id: id.to_string(),
            label: label.to_string(),
            icons,
            file: manifest.file,
            folder: manifest.folder,
            file_extensions: lower(manifest.file_extensions),
            file_names: lower(manifest.file_names),
            folder_names: lower(manifest.folder_names),
        }
    }

    /// VS Code's order: exact name, then the longest extension ("d.ts"
    /// before "ts"), then the pack's default file or folder icon.
    pub fn icon_for(&self, name: &str, is_dir: bool) -> Option<&PackSvg> {
        let name = name.to_lowercase();
        let def_id = if is_dir {
            self.folder_names.get(&name).or(self.folder.as_ref())
        } else {
            self.file_names
                .get(&name)
                .or_else(|| {
                    name.match_indices('.')
                        .map(|(i, _)| &name[i + 1..])
                        .find_map(|ext| self.file_extensions.get(ext))
                })
                .or(self.file.as_ref())
        }?;
        self.icons.get(def_id)
    }
}

/// Every pack, bundled first.
pub fn packs() -> &'static [IconPack] {
    &PACKS
}

/// The pack with `id`; `""` means no pack.
pub fn find(id: &str) -> Option<&'static IconPack> {
    if id.is_empty() {
        return None;
    }
    PACKS.iter().find(|p| p.id == id)
}

/// Paints `svg` centered in `rect`. Returns false when it can't be read, so
/// the caller falls back to the shell icon.
pub fn paint(ui: &egui::Ui, rect: Rect, size: f32, svg: &'static PackSvg, tint: Color32) -> bool {
    let Some(bytes) = svg.bytes() else {
        return false;
    };
    let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(size));
    Image::from_bytes(svg.uri.as_str(), Bytes::Shared(bytes))
        .fit_to_exact_size(Vec2::splat(size))
        .tint(tint)
        .paint_at(ui, icon_rect);
    true
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct PackageJson {
    contributes: Contributes,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Contributes {
    icon_themes: Vec<ThemeEntry>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ThemeEntry {
    id: String,
    label: String,
    path: String,
}

fn load_user_packs(dir: &Path) -> Vec<IconPack> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut folders: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    folders.sort();
    folders.iter().flat_map(|f| load_folder(f)).collect()
}

fn load_folder(folder: &Path) -> Vec<IconPack> {
    let name = folder
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let root = if folder.join("extension").join("package.json").is_file() {
        folder.join("extension")
    } else {
        folder.to_path_buf()
    };
    let themes = match read_json::<PackageJson>(&root.join("package.json")) {
        Some(package) => package.contributes.icon_themes,
        None => vec![ThemeEntry {
            id: "theme".to_string(),
            label: name.clone(),
            path: "theme.json".to_string(),
        }],
    };
    themes
        .into_iter()
        .filter_map(|theme| {
            let path = root.join(&theme.path);
            let manifest = read_json::<Manifest>(&path)?;
            // Labels like "%theme.label%" point into package.nls.json.
            let label = if theme.label.is_empty() || theme.label.starts_with('%') {
                format!("{name} ({})", theme.id)
            } else {
                theme.label
            };
            let id = format!("user:{name}/{}", theme.id);
            Some(IconPack::new(&id, &label, manifest, path.parent()))
        })
        .collect()
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mocha() -> &'static IconPack {
        find("catppuccin-mocha").expect("bundled pack")
    }

    fn icon_name(svg: &PackSvg) -> &str {
        svg.uri.rsplit('/').next().unwrap()
    }

    #[test]
    fn bundled_packs_load_with_inline_svgs() {
        for id in ["catppuccin-mocha", "catppuccin-latte"] {
            let pack = find(id).unwrap();
            assert!(pack.icons.len() > 500, "{id}: {}", pack.icons.len());
            assert!(pack.icons.values().all(|svg| svg.bytes().is_some()));
        }
    }

    #[test]
    fn matches_names_then_longest_extension_then_default() {
        let pack = mocha();
        assert_eq!(
            icon_name(pack.icon_for("main.rs", false).unwrap()),
            "rust.svg"
        );
        assert_eq!(
            icon_name(pack.icon_for("MAIN.RS", false).unwrap()),
            "rust.svg"
        );
        assert_eq!(
            icon_name(pack.icon_for("Cargo.toml", false).unwrap()),
            icon_name(pack.icon_for("cargo.toml", false).unwrap())
        );
        assert_eq!(
            icon_name(pack.icon_for("unknown.zzzz", false).unwrap()),
            "_file.svg"
        );
        assert_eq!(
            icon_name(pack.icon_for("src", true).unwrap()),
            "folder_src.svg"
        );
        assert_eq!(
            icon_name(pack.icon_for("zzzz", true).unwrap()),
            "_folder.svg"
        );
    }

    #[test]
    fn empty_id_is_no_pack() {
        assert!(find("").is_none());
        assert!(find("missing").is_none());
    }
}
