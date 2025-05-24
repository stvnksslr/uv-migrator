// Only declare modules when building as a binary
#[cfg(not(feature = "python"))]
mod cli;
#[cfg(not(feature = "python"))]
mod error;
#[cfg(not(feature = "python"))]
mod migrators;
#[cfg(not(feature = "python"))]
mod models;
#[cfg(not(feature = "python"))]
mod utils;

// When building with python feature, use the library modules
#[cfg(feature = "python")]
use uv_migrator::{cli, execute_with_args};

use log::error;
use std::process::exit;

fn main() {
    if let Err(e) = run() {
        error!("Error: {}", e);
        exit(1);
    }
}

#[cfg(not(feature = "python"))]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    use env_logger::{Builder, Env};
    use log::info;

    // Initialize logger with default info level
    Builder::from_env(Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .format_target(false)
        .init();

    info!("Starting UV migrator...");

    // Check UV requirements before proceeding
    utils::uv::check_uv_requirements()?;

    // Run the CLI and get arguments
    let args = cli::run()?;

    info!("Migrating project at: {}", args.path.display());

    // Run the migration
    migrators::run_migration(
        &args.path,
        args.import_global_pip_conf,
        &args.import_index,
        args.merge_groups,
        !args.disable_restore,
    )?;

    info!("Migration completed successfully!");
    Ok(())
}

#[cfg(feature = "python")]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    // When building with python feature, use the library version
    let args = cli::run()?;
    execute_with_args(&args).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}
