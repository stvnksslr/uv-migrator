use log::{error, info};
use std::env;
use std::path::Path;
use std::process::exit;
use clap::{Arg, Command};

mod types;
mod utils;
mod migrator;

fn main() {
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    if let Err(e) = which::which("uv") {
        error!("The 'uv' command is not available. Please install uv and ensure it's in your PATH. Error: {}", e);
        exit(1);
    }

    let matches = Command::new("uv-migrator")
        .version("1.0")
        .about("Migrates Python projects to use uv")
        .arg(Arg::new("PATH")
            .help("The path to the project directory")
            .required(true)
            .index(1))
        .arg(Arg::new("import-global-pip-conf")
            .long("import-global-pip-conf")
            .help("Import extra index URLs from ~/.pip/pip.conf")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("import-index")
            .long("import-index")
            .help("Additional index URL to import")
            .action(clap::ArgAction::Append))
        .get_matches();

    let input_path = Path::new(matches.get_one::<String>("PATH").unwrap());
    let project_dir = if input_path.is_dir() {
        input_path.to_path_buf()
    } else {
        input_path.parent().unwrap_or(Path::new(".")).to_path_buf()
    };

    let import_global_pip_conf = matches.get_flag("import-global-pip-conf");
    let additional_index_urls: Vec<String> = matches.get_many::<String>("import-index")
        .map(|values| values.cloned().collect())
        .unwrap_or_default();

    match migrator::run_migration(&project_dir, import_global_pip_conf, &additional_index_urls) {
        Ok(_) => info!("Migration completed successfully"),
        Err(e) => {
            error!("Error during migration: {}", e);
            exit(1);
        }
    }
}