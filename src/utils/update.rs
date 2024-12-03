use log::info;
use self_update::cargo_crate_version;

pub fn update() -> Result<(), String> {
    info!("Checking for updates...");

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
