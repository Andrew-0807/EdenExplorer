// gui/mod.rs
pub mod backdrop;
pub mod dragdrop;
pub mod folder_icons;
pub mod icon_system;
pub mod selection;
pub mod theme;
pub mod theme_presets;
pub mod utils;
pub mod windows; // for your windows folder

// optionally re-export MainWindow
pub use windows::mainwindow::MainWindow;
