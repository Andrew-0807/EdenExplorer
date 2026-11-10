//! zoxide integration: reads and updates the user's zoxide database by
//! running the `zoxide` CLI. Every function here blocks on a child process, so
//! the UI must call them from a worker thread only.

use std::io::ErrorKind;
use std::os::windows::process::CommandExt;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};

/// Keeps the child from flashing a console window.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// How long the user must stay in a folder before auto-add records it.
pub const AUTO_ADD_DELAY: Duration = Duration::from_secs(1);

const UNKNOWN: u8 = 0;
const PRESENT: u8 = 1;
const MISSING: u8 = 2;

/// Whether `zoxide` can be launched, learned from the first spawn attempt.
static AVAILABILITY: AtomicU8 = AtomicU8::new(UNKNOWN);

/// One row of `zoxide query -l -s`.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub score: f64,
    pub path: PathBuf,
    /// Filled by `query_all`; `parse_query_output` leaves it true.
    pub exists: bool,
}

/// True once a spawn failed with "not found". Unknown counts as available.
pub fn is_missing() -> bool {
    AVAILABILITY.load(Ordering::Relaxed) == MISSING
}

fn run(args: &[&str], path: Option<&Path>) -> Option<Output> {
    if is_missing() {
        return None;
    }
    let mut command = Command::new("zoxide");
    command.args(args);
    if let Some(path) = path {
        command.arg(path);
    }
    match command.creation_flags(CREATE_NO_WINDOW).output() {
        Ok(output) => {
            AVAILABILITY.store(PRESENT, Ordering::Relaxed);
            Some(output)
        }
        Err(err) => {
            if err.kind() == ErrorKind::NotFound {
                AVAILABILITY.store(MISSING, Ordering::Relaxed);
            }
            None
        }
    }
}

/// Parses `zoxide query -l -s` output: a right-aligned score, then the path.
/// Lines that do not start with a number are skipped.
pub fn parse_query_output(text: &str) -> Vec<Entry> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let split = line.find(char::is_whitespace)?;
            let score = line[..split].parse::<f64>().ok()?;
            let path = line[split..].trim();
            (!path.is_empty()).then(|| Entry {
                score,
                path: PathBuf::from(path),
                exists: true,
            })
        })
        .collect()
}

/// The whole database, best match first, with `exists` checked on disk.
/// `None` when zoxide is not installed or the query failed.
pub fn query_all() -> Option<Vec<Entry>> {
    let output = run(&["query", "-l", "-s"], None)?;
    if !output.status.success() {
        return None;
    }
    let mut entries = parse_query_output(&String::from_utf8_lossy(&output.stdout));
    for entry in &mut entries {
        entry.exists = entry.path.is_dir();
    }
    Some(entries)
}

pub fn add(path: &Path) -> bool {
    run(&["add"], Some(path)).is_some_and(|output| output.status.success())
}

pub fn remove(path: &Path) -> bool {
    run(&["remove"], Some(path)).is_some_and(|output| output.status.success())
}

/// Adds `path` on a worker thread, ignoring the result.
pub fn add_in_background(path: PathBuf) {
    std::thread::spawn(move || {
        add(&path);
    });
}

/// A real folder location: a drive or UNC path, not `::MY_PC::`, the Recycle
/// Bin, or a network or WSL root.
pub fn is_addable_path(path: &Path) -> bool {
    path.is_absolute()
        && matches!(path.components().next(), Some(Component::Prefix(_)))
        && !path.to_string_lossy().starts_with("::")
}

/// Tracks how long the user has stayed in one folder, for auto-add.
#[derive(Default)]
pub struct DwellTracker {
    path: PathBuf,
    since: Option<Instant>,
    done: bool,
}

impl DwellTracker {
    /// Call each frame with the current folder. Returns the folder once it has
    /// been shown for `AUTO_ADD_DELAY`, and only once per visit.
    pub fn tick(&mut self, current: &Path, now: Instant) -> Option<PathBuf> {
        if self.since.is_none() || self.path != current {
            self.path = current.to_path_buf();
            self.since = Some(now);
            self.done = false;
            return None;
        }
        let since = self.since?;
        if !self.done && now.duration_since(since) >= AUTO_ADD_DELAY {
            self.done = true;
            return Some(self.path.clone());
        }
        None
    }

    /// True while a timer is running and the UI should wake up for it.
    pub fn pending(&self) -> bool {
        self.since.is_some() && !self.done
    }

    /// Forget the current visit, e.g. when the setting is turned off.
    pub fn reset(&mut self) {
        self.since = None;
        self.done = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_scores_and_paths() {
        let text = "  18.0 C:\\Users\\Andrew\\.agents\r\n  16.0 G:\\projects\\Eden Explorer\n";
        let entries = parse_query_output(text);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].score, 18.0);
        assert_eq!(entries[0].path, PathBuf::from(r"C:\Users\Andrew\.agents"));
        assert_eq!(entries[1].path, PathBuf::from(r"G:\projects\Eden Explorer"));
    }

    #[test]
    fn parsing_skips_junk_and_accepts_unpadded_scores() {
        let text = "\nwarning: something\n1.5 D:\\x\n   2 E:\\y  \n3.0\n";
        let entries = parse_query_output(text);
        let paths: Vec<_> = entries.iter().map(|e| e.path.clone()).collect();
        assert_eq!(paths, [PathBuf::from(r"D:\x"), PathBuf::from(r"E:\y")]);
        assert_eq!(entries[1].score, 2.0);
    }

    #[test]
    fn only_real_locations_are_addable() {
        assert!(is_addable_path(Path::new(r"C:\Users")));
        assert!(is_addable_path(Path::new(r"D:\")));
        assert!(is_addable_path(Path::new(r"\\wsl.localhost\Ubuntu\home")));
        assert!(!is_addable_path(Path::new("::MY_PC::")));
        assert!(!is_addable_path(Path::new("::RECYCLE_BIN::")));
        assert!(!is_addable_path(Path::new(r"\\wsl.localhost")));
        assert!(!is_addable_path(Path::new(r"relative\dir")));
    }

    #[test]
    fn dwell_fires_once_after_the_delay() {
        let start = Instant::now();
        let mut tracker = DwellTracker::default();
        let a = Path::new(r"C:\a");
        assert_eq!(tracker.tick(a, start), None);
        assert!(tracker.pending());
        assert_eq!(tracker.tick(a, start + Duration::from_millis(500)), None);
        assert_eq!(
            tracker.tick(a, start + AUTO_ADD_DELAY),
            Some(PathBuf::from(r"C:\a"))
        );
        assert!(!tracker.pending());
        assert_eq!(tracker.tick(a, start + AUTO_ADD_DELAY * 3), None);
    }

    #[test]
    fn dwell_restarts_when_the_folder_changes() {
        let start = Instant::now();
        let mut tracker = DwellTracker::default();
        let (a, b) = (Path::new(r"C:\a"), Path::new(r"C:\b"));
        tracker.tick(a, start);
        let moved = start + Duration::from_millis(900);
        assert_eq!(tracker.tick(b, moved), None);
        assert_eq!(tracker.tick(b, start + AUTO_ADD_DELAY), None);
        assert_eq!(
            tracker.tick(b, moved + AUTO_ADD_DELAY),
            Some(PathBuf::from(r"C:\b"))
        );
    }
}
