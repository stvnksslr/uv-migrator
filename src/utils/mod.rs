pub mod author;
pub mod file_tracker;
pub mod pip;
pub mod pyproject;
pub mod toml;
#[cfg(feature = "self_update")]
mod update;
mod uv;

// Export needed items
pub use author::update_authors;
pub use file_tracker::FileTrackerGuard;
pub use pip::parse_pip_conf;
pub use pyproject::update_pyproject_toml;
pub use pyproject::update_url;
#[cfg(feature = "self_update")]
pub use update::update;
pub use uv::check_uv_requirements;

pub mod version;
