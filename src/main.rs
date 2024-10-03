mod types;
mod utils;
mod migrator;

use log::{error, info};
use std::env;
use std::path::Path;
use std::process::exit;

fn main() {
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    if let Err(e) = which::which("uv") {
        error!("The 'uv' command is not available. Please install uv and ensure it's in your PATH. Error: {}", e);
        exit(1);
    }

    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        error!("Usage: {} <path>", args[0]);
        exit(1);
    }

    let input_path = Path::new(&args[1]);
    let project_dir = if input_path.is_dir() {
        input_path.to_path_buf()
    } else {
        input_path.parent().unwrap_or(Path::new(".")).to_path_buf()
    };

    match migrator::run_migration(&project_dir) {
        Ok(_) => info!("Migration completed successfully"),
        Err(e) => {
            error!("Error during migration: {}", e);
            exit(1);
        }
    }
}