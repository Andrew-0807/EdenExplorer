# herdr integration spec

herdr 0.9.5. Researched with read-only commands only (`--help`, `workspace list`, `pane list`, `status --json`).

## CLI used
All commands talk to the running herdr server over its socket and print JSON. They do not need a visible terminal: the server is a detached daemon, so new panes/tabs/workspaces exist even with no client attached. `--focus` makes an attached client switch to the new item.

| Action | Command |
|---|---|
| List workspaces | `herdr workspace list` -> `{"result":{"workspaces":[{"workspace_id","label","number","focused",...}]}}` |
| List panes (anchor lookup) | `herdr pane list` -> `{"result":{"panes":[{"pane_id","workspace_id","focused","cwd",...}]}}` |
| New workspace | `herdr workspace create --cwd <dir> --label <folder name> --focus` |
| New tab | `herdr tab create --workspace <id> --cwd <dir> --focus` |
| New pane | `herdr pane split <anchor pane id> --direction right --cwd <dir> --focus` |

Why: there is no "create pane in workspace" command. A pane is made by splitting an existing pane, so the anchor is the workspace's focused pane (else its first pane), taken from one `pane list` call. Tab listing is not needed since `tab create` takes a workspace id.

## No server
Errors come back as JSON: `{"error":{"code":"server_not_running",...}}` (seen for `workspace list` against an unused session name). The menu then shows "New herdr workspace here" disabled with the hint "herdr is not running". Starting a server from the app is not attempted (attaching a client with `herdr` starts one).

## Not installed
`herdr` spawn fails with NotFound, cached as missing, and the right-click menu is not shown.

## App behavior
- Left click on the terminal toolbar button: unchanged (wt/powershell/cmd).
- Right click: refresh runs on a worker thread; the menu shows the cached snapshot (first open shows "Loading herdr..." until the refresh repaints). Each action runs on a worker thread, then refreshes.
- Start dir is the current folder (home for My PC or a missing path).
- Code: `src/core/herdr.rs`, menu in `src/gui/windows/containers/itemviewer_navbar.rs` (`herdr_menu`).

## Not verified (live session untouched)
- Real `workspace create`, `tab create`, `pane split` output and that `--cwd` is honored for Windows paths.
- That `pane split` accepts `--focus` together with a positional pane id (help shows both).
- Create commands with no server (assumed the same `server_not_running` error).
