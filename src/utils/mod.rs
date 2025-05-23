pub mod file_ops;
pub mod pip;
pub mod pyproject;
pub mod toml;
pub mod uv;

// Utility modules
pub mod author;
pub mod build_system;
pub mod version;

// Feature-dependent modules
#[cfg(feature = "self_update")]
mod update;
#[cfg(feature = "self_update")]
pub use update::{check_for_updates, update};

// Re-export commonly used items
pub use pip::parse_pip_conf;
