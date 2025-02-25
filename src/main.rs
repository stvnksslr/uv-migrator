mod cli;
mod error;
mod migrators;
mod models;
mod utils;

use env_logger::{Builder, Env};
use log::error;
use std::process::exit;

#[cfg(feature = "self_update")]
fn update_check(args: &cli::Args) -> crate::error::Result<()> {
    if args.self_update {
        if let Err(e) = utils::update() {
            eprintln!("Update failed: {}", e);
        }
    } else if args.check_update {
        if let Err(e) = utils::check_for_updates() {
            eprintln!("Update check failed: {}", e);
        }
    }
    Ok(())
}

#[cfg(not(feature = "self_update"))]
fn update_check(_args: &cli::Args) -> crate::error::Result<()> {
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
    // Run the CLI and get arguments
    let args = cli::run()?;

    // Check for updates if requested via flags
    update_check(&args)?;

    Ok(())
}
