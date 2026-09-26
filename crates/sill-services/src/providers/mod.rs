//! The launcher providers: the M4 five (plan "Launcher v1": apps, calculator, settings,
//! windows, system) and the M9 four (files, clipboard, emoji, the web fallback; FINDINGS "M9
//! freeze"). Each implements `sill_launcher::Provider`; they live here, not in the pure
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
