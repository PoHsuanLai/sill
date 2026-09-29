//! The launcher providers: apps, calculator, settings, windows, system, files, clipboard, emoji and
//! the web fallback. Each implements `sill_launcher::Provider`; they live here, not in the pure
//! `sill-launcher`, because each reads a service or the system.

pub mod apps;
pub mod calculator;
pub mod clipboard;
pub mod emoji;
pub mod files;
pub mod settings;
pub mod system;
pub mod web;
pub mod windows;

pub use apps::{AppsProvider, row_app_id};
pub use calculator::{CALCULATOR_BUDGET, CalculatorProvider};
pub use clipboard::ClipboardProvider;
pub use emoji::EmojiProvider;
pub use files::FilesProvider;
pub use settings::SettingsProvider;
pub use system::SystemProvider;
pub use web::WebProvider;
pub use windows::WindowsProvider;
