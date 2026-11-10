//! Versioned, self-describing storage helpers.
//!
//! Settings and themes used to be written with postcard, which is positional:
//! adding, removing or reordering a field made every existing file unreadable,
//! so users lost their settings on update. Files are now JSON documents wrapped
//! in a small envelope (`{"format": .., "version": .., ...}`). Readers merge the
//! stored values onto the current defaults, so new fields get their default and
//! unknown fields (from a newer build) are ignored.
//!
//! All writes go through [`write_atomic`] so a crash or power loss mid-write can
//! never leave a half-written file behind.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Folder under `%LOCALAPPDATA%` that holds every EdenExplorer file.
pub const APP_DIR_NAME: &str = "ExplorerEden";

/// `%LOCALAPPDATA%\ExplorerEden`, or `None` when the known folder is unavailable.
pub fn data_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|base| base.join(APP_DIR_NAME))
}

/// Writes `bytes` to `path` atomically: the data goes to a temporary file in
/// the same folder, is flushed to disk, and then replaces `path` with a rename.
/// Readers see either the old or the new content, never a partial file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "path has no parent")
    })?;
    let file_name = path.file_name().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "path has no file name")
    })?;
    std::fs::create_dir_all(parent)?;

    let tmp = parent.join(format!(
        ".{}.{}.tmp",
        file_name.to_string_lossy(),
        std::process::id()
    ));

    let result = (|| {
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        // std::fs::rename replaces an existing destination on Windows
        // (MoveFileExW with MOVEFILE_REPLACE_EXISTING).
        std::fs::rename(&tmp, path)
    })();

    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// Recursively overlays `overlay` onto `base`. Objects are merged key by key,
/// every other value (including arrays) replaces the base value.
pub fn merge_json(base: &mut Value, overlay: Value) {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (key, value) in overlay_map {
                match base_map.get_mut(&key) {
                    Some(existing) => merge_json(existing, value),
                    None => {
                        base_map.insert(key, value);
                    }
                }
            }
        }
        (slot, value) => *slot = value,
    }
}

/// Result of [`from_value_with_defaults`].
pub struct Merged<T> {
    pub value: T,
    /// Top-level keys of the stored document that could not be applied (bad
    /// type or value). Their defaults were kept.
    pub rejected: Vec<String>,
}

/// Deserializes `stored` on top of `defaults`.
///
/// Missing fields keep their default, unknown fields are ignored. When the
/// merged document does not deserialize (for example one field has the wrong
/// type), the fields are applied one at a time and only the ones that fail are
/// dropped, so a single bad value never discards the rest of the user's data.
pub fn from_value_with_defaults<T>(defaults: &T, stored: Value) -> Merged<T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    let Ok(base) = serde_json::to_value(defaults) else {
        return Merged {
            value: defaults.clone(),
            rejected: Vec::new(),
        };
    };

    let mut merged = base.clone();
    merge_json(&mut merged, stored.clone());
    if let Ok(value) = serde_json::from_value::<T>(merged) {
        return Merged {
            value,
            rejected: Vec::new(),
        };
    }

    // Slow path: apply the stored fields one by one.
    let mut accepted = base;
    let mut rejected = Vec::new();
    if let Value::Object(fields) = stored {
        for (key, value) in fields {
            let mut deep = accepted.clone();
            merge_json(&mut deep, single_field(&key, value.clone()));
            if serde_json::from_value::<T>(deep.clone()).is_ok() {
                accepted = deep;
                continue;
            }

            // A deep merge can produce an invalid value for enums stored as
            // objects (two variants at once); try replacing the field instead.
            let mut shallow = accepted.clone();
            if let Value::Object(map) = &mut shallow {
                map.insert(key.clone(), value);
            }
            if serde_json::from_value::<T>(shallow.clone()).is_ok() {
                accepted = shallow;
            } else {
                rejected.push(key);
            }
        }
    }

    let value = serde_json::from_value::<T>(accepted).unwrap_or_else(|_| defaults.clone());
    Merged { value, rejected }
}

fn single_field(key: &str, value: Value) -> Value {
    let mut map = Map::new();
    map.insert(key.to_string(), value);
    Value::Object(map)
}

/// Why a versioned document could not be read.
#[derive(Debug, PartialEq, Eq)]
pub enum EnvelopeError {
    /// Not valid JSON, or not a JSON object.
    Malformed,
    /// Valid JSON with a different `format` tag (not one of our files).
    WrongFormat,
}

/// A parsed `{"format", "version", ...}` document.
#[derive(Debug)]
pub struct Envelope {
    pub version: u32,
    pub body: Map<String, Value>,
}

/// Parses a versioned JSON document and checks its `format` tag. `version` is
/// returned so callers can migrate older layouts; it defaults to 0 if absent.
pub fn parse_envelope(bytes: &[u8], format: &str) -> Result<Envelope, EnvelopeError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| EnvelopeError::Malformed)?;
    let Value::Object(mut body) = value else {
        return Err(EnvelopeError::Malformed);
    };
    match body.remove("format") {
        Some(Value::String(tag)) if tag == format => {}
        _ => return Err(EnvelopeError::WrongFormat),
    }
    let version = body
        .remove("version")
        .and_then(|v| v.as_u64())
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(0);
    Ok(Envelope { version, body })
}

/// Serializes `body` (which must serialize to a JSON object) with a
/// `format`/`version` header, pretty-printed so users can read and diff it.
pub fn envelope_bytes<B: Serialize>(format: &str, version: u32, body: &B) -> Option<Vec<u8>> {
    let mut map = Map::new();
    map.insert("format".to_string(), Value::String(format.to_string()));
    map.insert("version".to_string(), Value::from(version));
    match serde_json::to_value(body).ok()? {
        Value::Object(fields) => map.extend(fields),
        _ => return None,
    }
    serde_json::to_vec_pretty(&Value::Object(map)).ok()
}

/// Keeps a copy of a file that could not be parsed next to it
/// (`name.unreadable`), so a bad file is never silently lost when a fresh one
/// is written over it.
pub fn preserve_unreadable(path: &Path) {
    let Some(name) = path.file_name() else {
        return;
    };
    let backup = path.with_file_name(format!("{}.unreadable", name.to_string_lossy()));
    if !backup.exists() {
        let _ = std::fs::copy(path, backup);
    }
}

#[cfg(test)]
pub(crate) mod test_dir {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};

    /// A unique, empty folder under the system temp dir, removed on drop.
    /// Tests use it instead of `%LOCALAPPDATA%` so they never touch real files.
    pub struct TempDir(PathBuf);

    impl TempDir {
        pub fn new(tag: &str) -> Self {
            static COUNTER: AtomicU32 = AtomicU32::new(0);
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "eden-test-{tag}-{}-{n}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0)
            ));
            std::fs::create_dir_all(&path).expect("create temp dir");
            Self(path)
        }

        pub fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_dir::TempDir;
    use super::*;
    use serde::Deserialize;

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    enum Mode {
        Off,
        Custom { width: f32 },
        Other { name: String },
    }

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct Sample {
        a: u32,
        b: String,
        mode: Mode,
        list: Vec<u8>,
    }

    fn defaults() -> Sample {
        Sample {
            a: 1,
            b: "default".into(),
            mode: Mode::Custom { width: 10.0 },
            list: vec![1, 2, 3],
        }
    }

    #[test]
    fn missing_fields_take_defaults_and_unknown_fields_are_ignored() {
        let stored = serde_json::json!({ "a": 7, "future_field": true });
        let merged = from_value_with_defaults(&defaults(), stored);
        assert!(merged.rejected.is_empty());
        assert_eq!(merged.value, Sample { a: 7, ..defaults() });
    }

    #[test]
    fn one_bad_field_does_not_discard_the_others() {
        let stored = serde_json::json!({ "a": "not a number", "b": "kept", "list": [9] });
        let merged = from_value_with_defaults(&defaults(), stored);
        assert_eq!(merged.rejected, vec!["a".to_string()]);
        assert_eq!(merged.value.a, 1);
        assert_eq!(merged.value.b, "kept");
        assert_eq!(merged.value.list, vec![9]);
    }

    #[test]
    fn enum_stored_as_a_different_object_variant_replaces_the_default() {
        let stored = serde_json::json!({ "mode": { "Other": { "name": "x" } } });
        let merged = from_value_with_defaults(&defaults(), stored);
        assert!(merged.rejected.is_empty());
        assert_eq!(merged.value.mode, Mode::Other { name: "x".into() });

        let stored = serde_json::json!({ "mode": "Off" });
        let merged = from_value_with_defaults(&defaults(), stored);
        assert_eq!(merged.value.mode, Mode::Off);
    }

    #[test]
    fn arrays_are_replaced_not_merged() {
        let stored = serde_json::json!({ "list": [] });
        let merged = from_value_with_defaults(&defaults(), stored);
        assert!(merged.value.list.is_empty());
    }

    #[test]
    fn envelope_round_trip_and_format_check() {
        let bytes = envelope_bytes("eden-test", 3, &defaults()).unwrap();
        let envelope = parse_envelope(&bytes, "eden-test").unwrap();
        assert_eq!(envelope.version, 3);
        assert_eq!(envelope.body.get("a"), Some(&Value::from(1)));
        assert!(!envelope.body.contains_key("format"));

        assert_eq!(
            parse_envelope(&bytes, "something-else").unwrap_err(),
            EnvelopeError::WrongFormat
        );
        assert_eq!(
            parse_envelope(b"\x00\x01garbage", "eden-test").unwrap_err(),
            EnvelopeError::Malformed
        );
        assert_eq!(
            parse_envelope(b"[1,2]", "eden-test").unwrap_err(),
            EnvelopeError::Malformed
        );
    }

    #[test]
    fn write_atomic_replaces_content_and_leaves_no_temp_file() {
        let dir = TempDir::new("atomic");
        let path = dir.path().join("nested").join("file.json");
        write_atomic(&path, b"first").unwrap();
        write_atomic(&path, b"second").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"second");

        let leftovers: Vec<_> = std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|name| name.ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "temp files left: {leftovers:?}");
    }

    #[test]
    fn write_atomic_failure_keeps_the_old_file() {
        let dir = TempDir::new("atomic-fail");
        let path = dir.path().join("file.json");
        write_atomic(&path, b"old").unwrap();
        // A directory with the destination's name cannot be replaced by a file.
        let blocked = dir.path().join("blocked");
        std::fs::create_dir_all(blocked.join("inner")).unwrap();
        assert!(write_atomic(&blocked, b"new").is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"old");
        assert!(blocked.join("inner").is_dir());
    }

    #[test]
    fn preserve_unreadable_copies_once() {
        let dir = TempDir::new("unreadable");
        let path = dir.path().join("theme.json");
        std::fs::write(&path, b"broken").unwrap();
        preserve_unreadable(&path);
        std::fs::write(&path, b"broken again").unwrap();
        preserve_unreadable(&path);
        assert_eq!(
            std::fs::read(dir.path().join("theme.json.unreadable")).unwrap(),
            b"broken"
        );
    }
}
