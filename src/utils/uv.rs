use log::info;
use semver::Version;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Minimum required UV version
const MIN_UV_VERSION: &str = "0.5.0";

/// Version that supports the --bare flag
pub const UV_SUPPORT_BARE: &str = "0.6.0";

/// Helper function to find the UV executable and ensure it meets version requirements
pub fn find_uv_path() -> Result<PathBuf, String> {
    // Check if uv is in PATH
    which::which("uv").map_err(|e| format!(
        "The 'uv' command is not available. Please install uv and ensure it's in your PATH. Error: {}",
        e
    ))
}

/// Gets the current UV version
///
/// Returns a semver Version that can be compared to determine if
/// we should use certain flags like --bare
pub fn get_uv_version() -> Result<Version, String> {
    let uv_path = find_uv_path()?;

    // Get the version by executing uv --version
    let output = Command::new(&uv_path)
        .arg("--version")
        .output()
        .map_err(|e| format!("Failed to execute uv --version: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to get uv version: {}", stderr));
    }

    let version_output = String::from_utf8_lossy(&output.stdout);
    log::debug!("Raw UV version output: {}", version_output);

    // The mock outputs "uv X.Y.Z" directly, so we extract the last part
    let version_str = version_output
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| format!("Unexpected uv version format: '{}'", version_output))?;

    log::debug!("Parsed version string: {}", version_str);

    Version::parse(version_str)
        .map_err(|e| format!("Failed to parse uv version '{}': {}", version_str, e))
}

/// Command builder for UV operations
pub struct UvCommandBuilder {
    uv_path: PathBuf,
    args: Vec<String>,
    working_dir: Option<PathBuf>,
}

impl UvCommandBuilder {
    /// Create a new command builder with the UV executable
    pub fn new() -> Result<Self, String> {
        let uv_path = find_uv_path()?;
        Ok(Self {
            uv_path,
            args: Vec::new(),
            working_dir: None,
        })
    }

    /// Add an argument to the command
    pub fn arg<S: Into<String>>(mut self, arg: S) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Add multiple arguments to the command
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for arg in args {
            self.args.push(arg.into());
        }
        self
    }

    /// Set the working directory for the command
    pub fn working_dir<P: AsRef<Path>>(mut self, dir: P) -> Self {
        self.working_dir = Some(dir.as_ref().to_path_buf());
        self
    }

    /// Execute the command and return the output
    pub fn execute(self) -> Result<Output, String> {
        let mut command = Command::new(&self.uv_path);
        command.args(&self.args);

        if let Some(dir) = self.working_dir {
            command.current_dir(dir);
        }

        info!("Executing UV command: {:?}", self.args);
        command
            .output()
            .map_err(|e| format!("Failed to execute UV command: {}", e))
    }

    /// Execute the command and check for success
    pub fn execute_success(self) -> Result<(), String> {
        let output = self.execute()?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("UV command failed: {}", stderr))
        }
    }
}

pub fn check_uv_requirements() -> Result<(), String> {
    let _uv_path = find_uv_path()?;

    // If uv is found, check its version
    let current_version = get_uv_version()?;

    let min_version = Version::parse(MIN_UV_VERSION)
        .map_err(|e| format!("Failed to parse minimum version: {}", e))?;

    if current_version < min_version {
        return Err(format!(
            "uv version {} or higher is required. Found version {}",
            MIN_UV_VERSION, current_version
        ));
    }

    Ok(())
}
