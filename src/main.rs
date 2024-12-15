use crate::utils::check_uv_requirements;
use clap::{Arg, Command};
use log::{error, info};
use std::env;
use std::path::Path;
use std::process::exit;

mod migrators;
mod types;
mod utils;

fn main() {
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    if let Err(e) = run() {
        error!("{}", e);
        exit(1);
    }
}

fn run() -> Result<(), String> {
    let matches = Command::new("uv-migrator")
        .version(env!("CARGO_PKG_VERSION"))
        .about("A tool for migrating Python projects to use the uv package manager")
        .long_about(
            "UV Migrator helps you convert Python projects from various dependency management systems \
            (like Poetry or pip) to use the UV package manager. It preserves your dependencies, \
            development configurations, and project structure while setting up a new UV-based environment."
        )
        .arg(
            Arg::new("PATH")
                .help("The path to the project directory to migrate")
                .long_help(
                    "Specifies the directory containing the Python project to migrate. \
                    This should be the root directory of your project where pyproject.toml \
                    or requirements.txt is located."
                )
                .required_unless_present("self-update")
                .value_parser(clap::value_parser!(String))
        )
        .arg(
            Arg::new("merge-groups")
                .long("merge-groups")
                .help("Merge all dependency groups into the dev group")
                .long_help(
                    "When this flag is set, all dependency groups (including custom groups) \
                    will be merged into the dev group. This is useful when you want to \
                    simplify your dependency management by having only main and dev dependencies."
                )
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("import-global-pip-conf")
                .long("import-global-pip-conf")
                .help("Import extra index URLs from ~/.pip/pip.conf")
                .long_help(
                    "Reads and imports any extra package index URLs defined in your global pip \
                    configuration file (~/.pip/pip.conf). This is useful when your project requires \
                    packages from private or alternative Python package indexes."
                )
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("import-index")
                .long("import-index")
                .help("Additional index URL to import")
                .long_help(
                    "Specifies additional Python package index URLs to use. You can provide this \
                    option multiple times to add several index URLs. These URLs will be added to \
                    your project's pyproject.toml in the [tool.uv] section."
                )
                .action(clap::ArgAction::Append)
                .value_parser(clap::value_parser!(String))
        )
        .arg(
            Arg::new("self-update")
                .long("self-update")
                .help("Update uv-migrator to the latest version")
                .long_help(
                    "Checks for and downloads the latest version of uv-migrator from GitHub releases. \
                    The tool will automatically update itself if a newer version is available."
                )
                .action(clap::ArgAction::SetTrue)
        )
        .after_help(
            "EXAMPLES:\n\
            # Migrate a project in the current directory\n\
            uv-migrator .\n\
            \n\
            # Merge all dependency groups into dev dependencies\n\
            uv-migrator . --merge-groups\n\
            \n\
            # Migrate a project with a private package index\n\
            uv-migrator . --import-index https://private.pypi.org/simple/\n\
            \n\
            # Migrate using global pip configuration\n\
            uv-migrator . --import-global-pip-conf\n\
            \n\
            # Update uv-migrator to the latest version\n\
            uv-migrator --self-update\n\
            \n\
            For more information and documentation, visit:\n\
            https://github.com/stvnksslr/uv-migrator"
        )
        .get_matches();

    if matches.get_flag("self-update") {
        return match utils::update() {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to update: {}", e)),
        };
    }

    if !matches.contains_id("PATH") {
        return Err("No path provided. Use --help for usage information.".to_string());
    }

    check_uv_requirements()?;

    let input_path = Path::new(matches.get_one::<String>("PATH").unwrap());
    let project_dir = if input_path.is_dir() {
        input_path.to_path_buf()
    } else {
        input_path.parent().unwrap_or(Path::new(".")).to_path_buf()
    };

    let import_global_pip_conf = matches.get_flag("import-global-pip-conf");
    let merge_groups = matches.get_flag("merge-groups");
    let additional_index_urls: Vec<String> = matches
        .get_many::<String>("import-index")
        .map(|values| values.cloned().collect())
        .unwrap_or_default();

    match migrators::run_migration(
        &project_dir,
        import_global_pip_conf,
        &additional_index_urls,
        merge_groups,
    ) {
        Ok(_) => {
            info!("Migration completed successfully");
            Ok(())
        }
        Err(e) => Err(format!("Migration failed: {}", e)),
    }
}
