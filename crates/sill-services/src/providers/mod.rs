//! The five launcher providers (plan "Launcher v1": apps, calculator, settings, windows,
//! system). Each implements `sill_launcher::Provider`; they live here, not in the pure
//! `sill-launcher`, because each reads a service or the system.

pub mod apps;
pub mod calculator;
pub mod settings;
pub mod system;
pub mod windows;

pub use apps::{AppsProvider, row_app_id};
pub use calculator::{CALCULATOR_BUDGET, CalculatorProvider};
pub use settings::SettingsProvider;
pub use system::SystemProvider;
pub use windows::WindowsProvider;
