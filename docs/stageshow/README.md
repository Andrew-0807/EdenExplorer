# Stageshow harness

Renders real EdenExplorer UI to PNG with no visible window (works in Windows session 0).
Backend: `egui_kittest` 0.35 with wgpu (a GPU/WARP adapter was available, no env vars needed).

## Run

    $env:CARGO_TARGET_DIR="D:/cargo-targets/stageshow"; $env:STAGESHOW="1"
    cargo test stageshow -- --nocapture

Without `STAGESHOW` the scene tests return immediately, so plain `cargo test` needs no GPU.
PNGs land in `docs/stageshow/<name>.png` (1.0 ppp: 1 point = 1 pixel; pass another `ppp` for 1.5 etc).

## Add a scene

Code: `src/gui/stageshow.rs` (test-only module). Copy a test and change the body:

    #[test]
    fn stageshow_my_scene() {
        if !enabled() { return; }
        let mode = ThemeMode::Dark;
        render_scene("my_scene", [w, h], 1.0, mode, get_palette(mode), move |ui, palette| {
            // draw into `ui`; windows via `ui.ctx()`, e.g. draw_settings_window(ui.ctx(), ...)
        });
    }

`render_scene` applies the theme, installs image loaders (needed for `AppIcon`), runs 4 frames and saves the PNG.
Shell icons (Windows file icons) are not available in the harness; use `AppIcon` and `IconStyle::Flat` folders.
Coupled containers (item viewer, explorer) need `MainWindow` state, so build lists from primitives with palette tokens.
