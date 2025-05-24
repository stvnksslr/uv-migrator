// Declare modules for the library build
pub mod cli;
pub mod error;
pub mod migrators;
pub mod models;
pub mod utils;

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
#[pyfunction]
fn run_cli(py: Python, args: Vec<String>) -> PyResult<()> {
    use std::ffi::OsString;

    // Release the GIL while running the CLI
    py.allow_threads(|| {
        // Prepare arguments for the CLI
        let mut cli_args = vec![OsString::from("uv-migrator")];
        cli_args.extend(args.into_iter().map(OsString::from));

        // Call the main CLI function with our arguments
        match run_main_with_args(cli_args) {
            Ok(_) => Ok(()),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    })
}

#[cfg(feature = "python")]
#[pymodule]
fn _uv_migrator(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(run_cli, m)?)?;
    Ok(())
}

// Function to run main with custom args (used by Python interface)
pub fn run_main_with_args(args: Vec<std::ffi::OsString>) -> crate::error::Result<()> {
    use crate::cli::Args;
    use std::path::PathBuf;

    // Parse the arguments manually - this is a basic implementation
    let mut path = PathBuf::from(".");
    let mut merge_groups = false;
    let mut import_global_pip_conf = false;
    let mut import_index = vec![];
    let mut disable_restore = false;
    #[cfg(feature = "self_update")]
    let mut self_update = false;
    #[cfg(feature = "self_update")]
    let mut check_update = false;

    // Simple argument parsing - you might want to make this more sophisticated
    let mut i = 1;
    while i < args.len() {
        let arg = args[i].to_string_lossy();
        match arg.as_ref() {
            "--merge-groups" => merge_groups = true,
            "--import-global-pip-conf" => import_global_pip_conf = true,
            "--disable-restore" => disable_restore = true,
            #[cfg(feature = "self_update")]
            "--self-update" => self_update = true,
            #[cfg(feature = "self_update")]
            "--check-update" => check_update = true,
            "--import-index" => {
                if i + 1 < args.len() {
                    i += 1;
                    import_index.push(args[i].to_string_lossy().to_string());
                }
            }
            _ => {
                if !arg.starts_with("-") {
                    path = PathBuf::from(arg.as_ref());
                }
            }
        }
        i += 1;
    }

    let cli_args = Args {
        path,
        merge_groups,
        import_global_pip_conf,
        import_index,
        disable_restore,
        #[cfg(feature = "self_update")]
        self_update,
        #[cfg(feature = "self_update")]
        check_update,
    };

    execute_with_args(&cli_args)
}

// Expose execute function for library use
pub fn execute_with_args(args: &cli::Args) -> crate::error::Result<()> {
    use env_logger::{Builder, Env};

    // Initialize logger with default info level
    Builder::from_env(Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .format_target(false)
        .init();

    #[cfg(feature = "self_update")]
    fn update_check(args: &cli::Args) -> crate::error::Result<()> {
        if args.self_update {
            if let Err(e) = crate::utils::update() {
                eprintln!("Update failed: {}", e);
            }
        } else if args.check_update {
            if let Err(e) = crate::utils::check_for_updates() {
                eprintln!("Update check failed: {}", e);
            }
        }
        Ok(())
    }

    #[cfg(not(feature = "self_update"))]
    fn update_check(_args: &cli::Args) -> crate::error::Result<()> {
        Ok(())
    }

    // Check for updates if requested via flags
    update_check(args)?;

    // Run the actual migration
    migrators::run_migration(
        &args.path,
        args.import_global_pip_conf,
        &args.import_index,
        args.merge_groups,
        !args.disable_restore,
    )
}
