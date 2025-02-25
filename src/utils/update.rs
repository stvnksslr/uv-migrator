use log::info;
use self_update::cargo_crate_version;

/// Checks if a newer version is available without updating
pub fn check_for_updates() -> Result<bool, String> {
    info!("Checking for updates...");

    let updater = self_update::backends::github::Update::configure()
        .repo_owner("stvnksslr")
        .repo_name("uv-migrator")
        .bin_name("uv-migrator")
        .current_version(cargo_crate_version!())
        .build()
        .map_err(|e| format!("Failed to build updater: {}", e))?;

    let latest_release = updater
        .get_latest_release()
        .map_err(|e| format!("Failed to check for updates: {}", e))?;

    // Compare versions
    let current_version = cargo_crate_version!();
    let update_available = latest_release.version != current_version;

    if update_available {
        info!("New version available: {}", latest_release.version);
    } else {
        info!(
            "No updates available. Already at latest version: {}",
            current_version
        );
    }

    Ok(update_available)
}

/// Downloads and applies the update
pub fn update() -> Result<(), String> {
    info!("Updating to the latest version...");

    let status = self_update::backends::github::Update::configure()
        .repo_owner("stvnksslr")
        .repo_name("uv-migrator")
        .bin_name("uv-migrator")
        .current_version(cargo_crate_version!())
        .build()
        .map_err(|e| format!("Failed to build updater: {}", e))?
        .update()
        .map_err(|e| format!("Failed to update binary: {}", e))?;

    match status.updated() {
        true => {
            info!(
                "Updated successfully to version {}! Please restart.",
                status.version()
            );
            Ok(())
        }
        false => {
            info!("No updates available. Already at latest version.");
            Ok(())
        }
    }
}
