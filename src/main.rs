mod cli;
mod error;
mod migrators;
mod models;
mod utils;

use env_logger::{Builder, Env};
use log::error;
use std::process::exit;

#[cfg(feature = "self_update")]
fn update_check() -> crate::error::Result<()> {
    let should_update = std::env::var("UV_MIGRATOR_UPDATE")
        .map(|v| v != "0")
        .unwrap_or(true);
    if should_update {
        if let Err(e) = utils::update() {
            eprintln!("Update check failed: {}", e);
        }
    }
    Ok(())
}

#[cfg(not(feature = "self_update"))]
fn update_check() -> crate::error::Result<()> {
    Ok(())
}

fn main() {
    // Initialize logger with default info level
    Builder::from_env(Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .format_target(false)
        .init();

    if let Err(e) = run() {
        error!("Error: {}", e);
        exit(1);
    }
}

fn run() -> crate::error::Result<()> {
    // Check for updates if enabled
    update_check()?;

    // Run the CLI
    cli::run()
}
