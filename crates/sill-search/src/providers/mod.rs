//! The launcher providers: apps, calculator, settings, windows, system, files, clipboard, emoji,
//! the web fallback, things, the companion, helpers, runs and memory. Each implements
//! `sill_launcher::Provider`; they live here, not in the pure `sill-launcher`, because each reads
//! a service or the system.

pub mod apps;
pub mod calculator;
pub mod clipboard;
pub mod companion;
pub mod emoji;
pub mod files;
pub mod helpers;
mod lookup;
pub mod memory;
pub mod runs;
pub mod settings;
pub mod system;
pub mod things;
pub mod web;
pub mod windows;

pub use apps::{AppsProvider, row_app_id};
pub use calculator::{CALCULATOR_BUDGET, CalculatorProvider};
pub use clipboard::ClipboardProvider;
pub use companion::CompanionProvider;
pub use emoji::EmojiProvider;
pub use files::FilesProvider;
pub use helpers::HelpersProvider;
pub use memory::MemoryProvider;
pub use runs::RunsProvider;
pub use settings::SettingsProvider;
pub use system::SystemProvider;
pub use things::ThingsProvider;
pub use web::WebProvider;
pub use windows::WindowsProvider;
