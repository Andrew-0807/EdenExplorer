//! herdr integration: lists workspaces and creates panes, tabs and workspaces
//! by running the `herdr` CLI, which talks to the running herdr server over its
//! socket. Every function that spawns blocks on a child process, so the UI
//! reads the cached `snapshot()` and starts `refresh_in_background()` instead.

use std::io::ErrorKind;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use serde_json::Value;

/// Keeps the child from flashing a console window.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const UNKNOWN: u8 = 0;
const MISSING: u8 = 2;
const PRESENT: u8 = 1;

/// Whether `herdr` can be launched, learned from the first spawn attempt.
static AVAILABILITY: AtomicU8 = AtomicU8::new(UNKNOWN);
static REFRESHING: AtomicBool = AtomicBool::new(false);
static SNAPSHOT: Mutex<Snapshot> = Mutex::new(Snapshot::Unknown);

#[derive(Clone, Debug, PartialEq)]
pub struct Workspace {
    pub id: String,
    pub label: String,
    /// A pane in this workspace that a new pane can be split from.
    pub anchor_pane: Option<String>,
}

/// What the last refresh learned about herdr.
#[derive(Clone, Debug, PartialEq)]
pub enum Snapshot {
    /// No refresh has finished yet.
    Unknown,
    /// herdr answered but its server is not running.
    NoServer,
    Workspaces(Vec<Workspace>),
}

/// True once a spawn failed with "not found". Unknown counts as available.
pub fn is_missing() -> bool {
    AVAILABILITY.load(Ordering::Relaxed) == MISSING
}

fn run(args: &[&str]) -> Option<Output> {
    if is_missing() {
        return None;
    }
    match Command::new("herdr")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
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

fn parse(text: &str) -> Option<Value> {
    serde_json::from_str(text.trim()).ok()
}

fn is_no_server(value: &Value) -> bool {
    value.pointer("/error/code").and_then(Value::as_str) == Some("server_not_running")
}

/// Parses `herdr workspace list` and `herdr pane list` JSON into workspaces.
/// Returns `NoServer` when the server is not running, and `None` when the
/// workspace output is not understood.
pub fn parse_snapshot(workspaces_json: &str, panes_json: &str) -> Option<Snapshot> {
    let ws = parse(workspaces_json)?;
    if is_no_server(&ws) {
        return Some(Snapshot::NoServer);
    }
    let list = ws.pointer("/result/workspaces")?.as_array()?;
    let panes_value = parse(panes_json);
    let panes: &[Value] = panes_value
        .as_ref()
        .and_then(|p| p.pointer("/result/panes"))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    let workspaces = list
        .iter()
        .filter_map(|w| {
            let id = w.get("workspace_id")?.as_str()?.to_owned();
            let label = w
                .get("label")
                .and_then(Value::as_str)
                .filter(|l| !l.is_empty())
                .unwrap_or(&id)
                .to_owned();
            let in_ws =
                |p: &&Value| p.get("workspace_id").and_then(Value::as_str) == Some(id.as_str());
            let focused = panes
                .iter()
                .filter(in_ws)
                .find(|p| p.get("focused").and_then(Value::as_bool) == Some(true));
            let anchor_pane = focused
                .or_else(|| panes.iter().find(in_ws))
                .and_then(|p| p.get("pane_id")?.as_str())
                .map(str::to_owned);
            Some(Workspace {
                id,
                label,
                anchor_pane,
            })
        })
        .collect();
    Some(Snapshot::Workspaces(workspaces))
}

fn fetch() -> Option<Snapshot> {
    let ws = run(&["workspace", "list"])?;
    let ws_text = String::from_utf8_lossy(&ws.stdout);
    let panes_text = run(&["pane", "list"])
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    parse_snapshot(&ws_text, &panes_text)
}

/// The result of the last refresh.
pub fn snapshot() -> Snapshot {
    SNAPSHOT
        .lock()
        .map(|s| s.clone())
        .unwrap_or(Snapshot::Unknown)
}

/// Re-reads workspaces on a worker thread, then asks for a repaint. Does
/// nothing while a refresh is already running or herdr is not installed.
pub fn refresh_in_background(ctx: &egui::Context) {
    if is_missing() || REFRESHING.swap(true, Ordering::AcqRel) {
        return;
    }
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        if let Some(next) = fetch()
            && let Ok(mut slot) = SNAPSHOT.lock()
        {
            *slot = next;
        }
        REFRESHING.store(false, Ordering::Release);
        ctx.request_repaint();
    });
}

fn dir_label(dir: &Path) -> String {
    dir.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| dir.display().to_string())
}

pub fn new_workspace_args(dir: &Path) -> Vec<String> {
    vec![
        "workspace".into(),
        "create".into(),
        "--cwd".into(),
        dir.display().to_string(),
        "--label".into(),
        dir_label(dir),
        "--focus".into(),
    ]
}

pub fn new_tab_args(workspace_id: &str, dir: &Path) -> Vec<String> {
    vec![
        "tab".into(),
        "create".into(),
        "--workspace".into(),
        workspace_id.into(),
        "--cwd".into(),
        dir.display().to_string(),
        "--focus".into(),
    ]
}

pub fn new_pane_args(anchor_pane: &str, dir: &Path) -> Vec<String> {
    vec![
        "pane".into(),
        "split".into(),
        anchor_pane.into(),
        "--direction".into(),
        "right".into(),
        "--cwd".into(),
        dir.display().to_string(),
        "--focus".into(),
    ]
}

fn spawn_detached(args: Vec<String>, ctx: egui::Context) {
    std::thread::spawn(move || {
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        if let Some(out) = run(&refs)
            && !out.status.success()
        {
            eprintln!(
                "herdr {} failed: {}",
                args.first().map(String::as_str).unwrap_or(""),
                String::from_utf8_lossy(&out.stdout)
            );
        }
        refresh_in_background(&ctx);
    });
}

pub fn create_workspace(dir: &Path, ctx: &egui::Context) {
    spawn_detached(new_workspace_args(dir), ctx.clone());
}

pub fn create_tab(workspace_id: &str, dir: &Path, ctx: &egui::Context) {
    spawn_detached(new_tab_args(workspace_id, dir), ctx.clone());
}

pub fn create_pane(anchor_pane: &str, dir: &Path, ctx: &egui::Context) {
    spawn_detached(new_pane_args(anchor_pane, dir), ctx.clone());
}

#[cfg(test)]
mod tests {
    use super::*;

    const WS: &str = r#"{"id":"cli:workspace:list","result":{"type":"workspace_list","workspaces":[{"label":"server","number":1,"workspace_id":"wP"},{"label":"EdenExplorer","number":6,"workspace_id":"w0","focused":true},{"label":"","workspace_id":"wX"}]}}"#;
    const PANES: &str = r#"{"id":"cli:pane:list","result":{"panes":[{"pane_id":"wP:p1","focused":false,"workspace_id":"wP"},{"pane_id":"w0:pE","focused":false,"workspace_id":"w0"},{"pane_id":"w0:p8","focused":true,"workspace_id":"w0"}],"type":"pane_list"}}"#;

    #[test]
    fn parses_workspaces_with_anchor() {
        let Some(Snapshot::Workspaces(ws)) = parse_snapshot(WS, PANES) else {
            panic!("expected workspaces");
        };
        assert_eq!(ws.len(), 3);
        assert_eq!(ws[0].anchor_pane.as_deref(), Some("wP:p1"));
        assert_eq!(ws[1].anchor_pane.as_deref(), Some("w0:p8"));
        assert_eq!(ws[2].label, "wX");
        assert_eq!(ws[2].anchor_pane, None);
    }

    #[test]
    fn missing_pane_list_still_lists_workspaces() {
        let Some(Snapshot::Workspaces(ws)) = parse_snapshot(WS, "garbage") else {
            panic!("expected workspaces");
        };
        assert_eq!(ws.len(), 3);
        assert!(ws.iter().all(|w| w.anchor_pane.is_none()));
    }

    #[test]
    fn detects_no_server() {
        let err = r#"{"id":"x","error":{"code":"server_not_running","message":"no server"}}"#;
        assert_eq!(parse_snapshot(err, ""), Some(Snapshot::NoServer));
    }

    #[test]
    fn rejects_unknown_output() {
        assert_eq!(parse_snapshot("not json", ""), None);
        assert_eq!(parse_snapshot(r#"{"result":{}}"#, ""), None);
    }

    #[test]
    fn builds_args() {
        let dir = Path::new(r"C:\Users\a\proj");
        let args = new_workspace_args(dir);
        assert_eq!(args[..2], ["workspace", "create"]);
        assert_eq!(args[3], r"C:\Users\a\proj");
        assert_eq!(args[5], "proj");
        assert_eq!(
            new_tab_args("w0", dir)[..4],
            ["tab", "create", "--workspace", "w0"]
        );
        assert_eq!(new_pane_args("w0:p8", dir)[..3], ["pane", "split", "w0:p8"]);
    }
}
