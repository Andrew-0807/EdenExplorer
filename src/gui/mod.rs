// gui/mod.rs
pub mod backdrop;
pub mod customization;
pub mod dragdrop;
pub mod folder_icons;
pub mod icon_system;
pub mod keymap;
pub mod selection;
#[cfg(test)]
mod stageshow;
pub mod theme;
pub mod theme_presets;
pub mod toast;
pub mod utils;
pub mod windows; // for your windows folder

// optionally re-export MainWindow
pub use windows::mainwindow::MainWindow;
