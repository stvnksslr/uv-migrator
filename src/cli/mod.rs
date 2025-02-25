use crate::error::Result;
use crate::migrators::run_migration;
use crate::utils::uv::check_uv_requirements;
use clap::{Arg, ArgAction, Command};
use log::info;
use std::path::PathBuf;

/// Command line arguments for UV migrator
#[derive(Debug)]
pub struct Args {
    /// Path to the project directory
    pub path: PathBuf,

    /// Whether to merge dependency groups
    pub merge_groups: bool,

    /// Whether to import global pip.conf
    pub import_global_pip_conf: bool,

    /// Additional index URLs to import
    pub import_index: Vec<String>,

    /// Whether to disable automatic restore on error
    pub disable_restore: bool,

    /// Whether to self-update
    #[cfg(feature = "self_update")]
    pub self_update: bool,

    /// Whether to check for updates without updating
    #[cfg(feature = "self_update")]
    pub check_update: bool,
}

/// Configures and runs the CLI
pub fn run() -> Result<Args> {
    let mut cmd = Command::new("uv-migrator")
        .version(env!("CARGO_PKG_VERSION"))
        .about("A tool for migrating Python projects to use the uv package manager")
        .long_about(
            "UV Migrator helps you convert Python projects from various dependency management systems \
            (like Poetry or pip) to use the UV package manager. It preserves your dependencies, \
            development configurations, and project structure while setting up a new UV-based environment."
        );

    cmd = cmd.arg(
        Arg::new("PATH")
            .help("The path to the project directory to migrate")
            .long_help(
                "Specifies the directory containing the Python project to migrate. \
                This should be the root directory of your project where pyproject.toml \
                or requirements.txt is located.",
            )
            .value_parser(clap::value_parser!(PathBuf))
            .default_value("."),
    );

    cmd = cmd.arg(
        Arg::new("merge-groups")
            .long("merge-groups")
            .help("Merge all dependency groups into the dev group")
            .long_help(
                "When this flag is set, all dependency groups (including custom groups) \
                will be merged into the dev group. This is useful when you want to \
                simplify your dependency management by having only main and dev dependencies.",
            )
            .action(ArgAction::SetTrue),
    );

    cmd = cmd.arg(
        Arg::new("import-global-pip-conf")
            .long("import-global-pip-conf")
            .help("Import extra index URLs from ~/.pip/pip.conf")
            .long_help(
                "Reads and imports any extra package index URLs defined in your global pip \
                configuration file (~/.pip/pip.conf). This is useful when your project requires \
                packages from private or alternative Python package indexes.",
            )
            .action(ArgAction::SetTrue),
    );

    cmd = cmd.arg(
        Arg::new("import-index")
            .long("import-index")
            .help("Additional index URL to import")
            .long_help(
                "Specifies additional Python package index URLs to use. You can provide this \
                option multiple times to add several index URLs. These URLs will be added to \
                your project's pyproject.toml in the [tool.uv] section.",
            )
            .action(ArgAction::Append)
            .value_parser(clap::value_parser!(String)),
    );

    cmd = cmd.arg(
        Arg::new("disable-restore")
            .long("disable-restore")
            .help("Disable automatic file restore on error")
            .long_help(
                "When this flag is set, the migrator will not attempt to restore files to their \
                original state if an error occurs during migration. This can be useful in \
                automated environments or when you want to inspect the partial migration state.",
            )
            .action(ArgAction::SetTrue),
    );

    // Add self-update functionality if the feature is enabled
    #[cfg(feature = "self_update")]
    {
        cmd = cmd.arg(
            Arg::new("self_update")
                .long("self-update")
                .help("Update uv-migrator to the latest version")
                .long_help(
                    "Checks for and downloads the latest version of uv-migrator from GitHub releases. \
                    The tool will automatically update itself if a newer version is available."
                )
                .action(ArgAction::SetTrue)
        );

        cmd = cmd.arg(
            Arg::new("check_update")
                .long("check-update")
                .help("Check for updates without installing them")
                .long_help(
                    "Checks if a newer version of uv-migrator is available on GitHub releases, \
                    but does not install the update. Use --self-update to both check and install.",
                )
                .action(ArgAction::SetTrue),
        );
    }

    let after_help = "EXAMPLES:
# Migrate a project in the current directory
uv-migrator .

# Merge all dependency groups into dev dependencies
uv-migrator . --merge-groups

# Migrate a project with a private package index
uv-migrator . --import-index https://private.pypi.org/simple/

# Migrate using global pip configuration
uv-migrator . --import-global-pip-conf

# Migrate without automatic restore on error
uv-migrator . --disable-restore

# Check for updates without installing them
uv-migrator --check-update

# Update to the latest version
uv-migrator --self-update

For more information and documentation, visit:
https://github.com/stvnksslr/uv-migrator";

    cmd = cmd.after_help(after_help);

    let matches = cmd.get_matches();

    let args = Args {
        path: matches
            .get_one::<PathBuf>("PATH")
            .cloned()
            .unwrap_or_else(|| PathBuf::from(".")),
        merge_groups: matches.get_flag("merge-groups"),
        import_global_pip_conf: matches.get_flag("import-global-pip-conf"),
        import_index: matches
            .get_many::<String>("import-index")
            .unwrap_or_default()
            .cloned()
            .collect(),
        disable_restore: matches.get_flag("disable-restore"),
        #[cfg(feature = "self_update")]
        self_update: matches.get_flag("self_update"),
        #[cfg(feature = "self_update")]
        check_update: matches.get_flag("check_update"),
    };

    execute(&args)?;
    Ok(args)
}

/// Execute the migration with the provided arguments
pub fn execute(args: &Args) -> Result<()> {
    // If we're only checking for updates or doing a self-update,
    // we don't need to run the migration
    #[cfg(feature = "self_update")]
    if args.self_update || args.check_update {
        return Ok(());
    }

    info!("Starting UV migrator...");

    // Check UV requirements before proceeding
    check_uv_requirements()?;

    info!("Migrating project at: {}", args.path.display());

    // Run the migration
    run_migration(
        &args.path,
        args.import_global_pip_conf,
        &args.import_index,
        args.merge_groups,
        !args.disable_restore,
    )?;

    info!("Migration completed successfully!");
    Ok(())
}
