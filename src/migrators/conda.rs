use crate::error::{Error, Result};
use crate::migrators::MigrationSource;
use crate::models::dependency::{Dependency, DependencyType};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Represents a Conda environment.yml file structure
#[derive(Debug, Deserialize, Serialize)]
pub struct CondaEnvironment {
    name: Option<String>,
    channels: Option<Vec<String>>,
    dependencies: Option<Vec<CondaDependency>>,
}

/// Represents different types of dependencies in a Conda environment
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum CondaDependency {
    /// Simple string dependency (e.g., "numpy")
    Simple(String),
    /// Versioned dependency (e.g., "pandas=1.3.0")
    Versioned(String),
    /// Pip dependencies
    Pip(HashMap<String, Vec<String>>),
}

pub struct CondaMigrationSource;

impl CondaMigrationSource {
    /// Detects if a project uses Conda by checking for environment.yml or environment.yaml
    pub fn detect_project_type(project_dir: &Path) -> bool {
        project_dir.join("environment.yml").exists()
            || project_dir.join("environment.yaml").exists()
    }

    /// Extracts Python version from Conda environment file
    pub fn extract_python_version_from_environment(project_dir: &Path) -> Result<Option<String>> {
        let source = CondaMigrationSource;
        let env_file = source.find_environment_file(project_dir);
        if env_file.is_none() {
            return Ok(None);
        }

        let env_file = env_file.unwrap();
        let content = fs::read_to_string(&env_file).map_err(|e| Error::FileOperation {
            path: env_file.clone(),
            message: format!("Failed to read environment file: {}", e),
        })?;

        let env: CondaEnvironment = serde_yml::from_str(&content).map_err(|e| {
            Error::DependencyParsing(format!("Failed to parse Conda environment file: {}", e))
        })?;

        if let Some(dependencies) = env.dependencies {
            if let Some(version) = source.extract_python_version(&dependencies) {
                return Ok(Some(version));
            }
        }

        Ok(None)
    }

    /// Finds the environment file (supports both .yml and .yaml extensions)
    fn find_environment_file(&self, project_dir: &Path) -> Option<std::path::PathBuf> {
        let yml_path = project_dir.join("environment.yml");
        if yml_path.exists() {
            return Some(yml_path);
        }

        let yaml_path = project_dir.join("environment.yaml");
        if yaml_path.exists() {
            return Some(yaml_path);
        }

        None
    }

    /// Parses a Conda dependency string into name and version
    fn parse_conda_dependency(&self, dep_str: &str) -> (String, Option<String>) {
        // First try to match comparison operators (including compound ones like >=, <=, !=)
        // Package names can contain letters, numbers, hyphens, underscores, and dots
        let re = regex::Regex::new(r"^([a-zA-Z0-9\-_.]+)\s*([><=!]+)\s*(.+)$").unwrap();
        if let Some(captures) = re.captures(dep_str) {
            let name = captures.get(1).map(|m| m.as_str()).unwrap_or(dep_str);
            let op = captures.get(2).map(|m| m.as_str()).unwrap_or("");
            let version = captures.get(3).map(|m| m.as_str()).unwrap_or("");

            // Handle special operators
            match op {
                "=" => {
                    // Single = in conda means exact version (== in pip)
                    let pip_version = if version.contains('*') {
                        self.convert_wildcard_version(version)
                    } else {
                        Some(format!("=={}", version))
                    };
                    (name.to_string(), pip_version)
                }
                _ => {
                    // For other operators, pass through as-is
                    (name.to_string(), Some(format!("{}{}", op, version)))
                }
            }
        } else {
            // No version specified
            (dep_str.trim().to_string(), None)
        }
    }

    /// Converts Conda wildcard versions to pip-compatible version ranges
    fn convert_wildcard_version(&self, version: &str) -> Option<String> {
        if version == "*" {
            return None; // Any version
        }

        // Handle patterns like "1.2.*"
        if version.ends_with(".*") {
            let base = version.trim_end_matches(".*");
            let parts: Vec<&str> = base.split('.').collect();

            match parts.len() {
                1 => {
                    // "1.*" -> ">=1.0.0,<2.0.0"
                    let major: u32 = parts[0].parse().ok()?;
                    Some(format!(">={}.0.0,<{}.0.0", major, major + 1))
                }
                2 => {
                    // "1.2.*" -> ">=1.2.0,<1.3.0"
                    let major = parts[0];
                    let minor: u32 = parts[1].parse().ok()?;
                    Some(format!(
                        ">={}.{}.0,<{}.{}.0",
                        major,
                        minor,
                        major,
                        minor + 1
                    ))
                }
                _ => Some(version.replace('*', "0")),
            }
        } else {
            Some(version.to_string())
        }
    }

    /// Checks if a package should be skipped (non-Python packages)
    fn should_skip_package(&self, name: &str) -> bool {
        // Skip packages that start with underscore - these are typically conda-specific internal packages
        if name.starts_with('_') {
            return true;
        }

        // Skip common system packages and non-Python dependencies
        const SKIP_PACKAGES: &[&str] = &[
            "python",
            "pip",
            "setuptools",
            "wheel",
            // Common Conda-specific packages that aren't on PyPI
            "anaconda",
            "anaconda-client",
            "anaconda-navigator",
            "anaconda-project",
            "navigator-updater",
            "conda",
            "conda-build",
            "conda-env",
            "conda-pack",
            "conda-verify",
            "conda-package-handling",
            "clyent",
            "console_shortcut",
            "dask-core",
            "get_terminal_size",
            "matplotlib-base",
            "numpy-base",
            "path.py",
            "py-lief",
            "singledispatch",
            "blosc",
            "bkcharts",
            "tbb",
            "zope",
            "backports", // This is just a namespace package
            // System libraries often installed via Conda
            "bzip2",
            "curl",
            "freetype",
            "get_terminal_size",
            "hdf5",
            "icu",
            "jpeg",
            "krb5",
            "libarchive",
            "libcurl",
            "libiconv",
            "liblief",
            "libllvm9",
            "libpng",
            "libsodium",
            "libspatialindex",
            "libssh2",
            "libtiff",
            "libxml2",
            "libxslt",
            "lz4-c",
            "lzo",
            "mpc",
            "mpfr",
            "mpir",
            "pandoc",
            "qt",
            "sip",
            "snappy",
            "yaml",
            "zeromq",
            "zstd",
            "libgcc-ng",
            "libstdcxx-ng",
            "libffi",
            "openssl",
            "ca-certificates",
            "certifi",
            "ncurses",
            "readline",
            "sqlite",
            "tk",
            "xz",
            "zlib",
            "libedit",
            "libgomp",
            "_libgcc_mutex",
            "_openmp_mutex",
            "ld_impl_linux-64",
            "libgfortran-ng",
            "libgfortran4",
            "libgfortran5",
            "mkl",
            "mkl-service",
            "mkl_fft",
            "mkl_random",
            "intel-openmp",
            "blas",
            "openblas",
            "libopenblas",
            // R packages (if using r channel)
            "r-base",
            "r-essentials",
            // CUDA packages
            "cudatoolkit",
            "cudnn",
            "cuda",
            // Other build tools
            "make",
            "cmake",
            "gcc",
            "gxx",
            "gfortran",
            "compilers",
            "c-compiler",
            "cxx-compiler",
            "fortran-compiler",
        ];

        SKIP_PACKAGES.contains(&name)
    }

    /// Maps Conda package names to their PyPI equivalents
    fn map_conda_to_pypi_name(&self, conda_name: &str) -> String {
        // Map common Conda package names to PyPI names
        match conda_name {
            "pytorch" => "torch",
            "pytorch-cpu" => "torch",
            "pytorch-gpu" => "torch",
            "tensorflow-gpu" => "tensorflow",
            "py-opencv" => "opencv-python",
            "pillow-simd" => "pillow",
            "msgpack-python" => "msgpack",
            "protobuf3" => "protobuf",
            "pyqt" => "pyqt5",
            "pyyaml" => "PyYAML",
            "beautifulsoup4" => "beautifulsoup4",
            "lxml" => "lxml",
            "pytables" => "tables",
            "tensorflow-mkl" => "tensorflow",
            "ruamel_yaml" => "ruamel.yaml",
            "importlib_metadata" => "importlib-metadata",
            "prompt_toolkit" => "prompt-toolkit",
            _ => conda_name,
        }
        .to_string()
    }

    /// Updates old package versions that are known to have compatibility issues
    fn update_problematic_versions(&self, name: &str, version: Option<String>) -> Option<String> {
        // For packages with known build issues on newer Python versions,
        // suggest updated versions that maintain compatibility
        match name {
            "bokeh" => {
                if let Some(v) = &version {
                    // bokeh 2.1.1 has issues with Python 3.12+
                    if v == "==2.1.1" {
                        // Update to a version that works with newer Python
                        // but is still from a similar era for compatibility
                        info!(
                            "Updating bokeh from {} to ==2.4.3 for Python compatibility",
                            v
                        );
                        return Some("==2.4.3".to_string());
                    }
                }
                version
            }
            _ => version,
        }
    }

    /// Extracts Python version requirement from dependencies
    fn extract_python_version(&self, dependencies: &[CondaDependency]) -> Option<String> {
        for dep in dependencies {
            match dep {
                CondaDependency::Simple(s) | CondaDependency::Versioned(s) => {
                    let (name, version) = self.parse_conda_dependency(s);
                    if name == "python" {
                        return version.map(|v| {
                            // Strip version operator prefix if present
                            let version_str = v.strip_prefix("==").unwrap_or(&v);
                            // Extract major.minor version
                            let parts: Vec<&str> = version_str.split('.').collect();
                            if parts.len() >= 2 {
                                format!("{}.{}", parts[0], parts[1])
                            } else {
                                version_str.to_string()
                            }
                        });
                    }
                }
                _ => continue,
            }
        }
        None
    }

    /// Process pip dependencies from the environment file
    fn process_pip_dependencies(&self, pip_deps: &[String]) -> Vec<Dependency> {
        let mut dependencies = Vec::new();

        for dep_str in pip_deps {
            // Parse pip dependency format
            let dep_str = dep_str.trim();

            // Skip empty lines and comments
            if dep_str.is_empty() || dep_str.starts_with('#') {
                continue;
            }

            // Handle different pip dependency formats
            if dep_str.starts_with("-e") || dep_str.starts_with("--editable") {
                // Skip editable installs for now
                warn!("Skipping editable install: {}", dep_str);
                continue;
            }

            // Parse dependency with extras and version
            let (name, version, extras) = self.parse_pip_dependency(dep_str);

            dependencies.push(Dependency {
                name,
                version,
                dep_type: DependencyType::Main,
                environment_markers: None,
                extras,
            });
        }

        dependencies
    }

    /// Parse pip dependency string with potential extras
    fn parse_pip_dependency(&self, dep_str: &str) -> (String, Option<String>, Option<Vec<String>>) {
        // Handle dependencies with extras like "package[extra1,extra2]>=1.0.0"
        let extras_regex = regex::Regex::new(r"^([a-zA-Z0-9\-_]+)\[([^\]]+)](.*)$").unwrap();

        if let Some(captures) = extras_regex.captures(dep_str) {
            let name = captures.get(1).map(|m| m.as_str()).unwrap_or(dep_str);
            let extras_str = captures.get(2).map(|m| m.as_str()).unwrap_or("");
            let version_part = captures.get(3).map(|m| m.as_str()).unwrap_or("");

            let extras = extras_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>();

            let version = if version_part.is_empty() {
                None
            } else {
                Some(version_part.trim().to_string())
            };

            (name.to_string(), version, Some(extras))
        } else {
            // No extras, parse normally
            let version_regex = regex::Regex::new(r"^([a-zA-Z0-9\-_]+)(.*)$").unwrap();

            if let Some(captures) = version_regex.captures(dep_str) {
                let name = captures.get(1).map(|m| m.as_str()).unwrap_or(dep_str);
                let version_part = captures.get(2).map(|m| m.as_str()).unwrap_or("");

                let version = if version_part.is_empty() {
                    None
                } else {
                    Some(version_part.trim().to_string())
                };

                (name.to_string(), version, None)
            } else {
                (dep_str.to_string(), None, None)
            }
        }
    }
}

impl MigrationSource for CondaMigrationSource {
    fn extract_dependencies(&self, project_dir: &Path) -> Result<Vec<Dependency>> {
        info!("Extracting dependencies from Conda environment file");

        let env_file = self.find_environment_file(project_dir).ok_or_else(|| {
            Error::ProjectDetection("No environment.yml or environment.yaml file found".to_string())
        })?;

        let content = fs::read_to_string(&env_file).map_err(|e| Error::FileOperation {
            path: env_file.clone(),
            message: format!("Failed to read environment file: {}", e),
        })?;

        let env: CondaEnvironment = serde_yml::from_str(&content).map_err(|e| {
            Error::DependencyParsing(format!("Failed to parse Conda environment file: {}", e))
        })?;

        let mut dependencies = Vec::new();

        if let Some(conda_deps) = env.dependencies {
            debug!("Processing {} Conda dependencies", conda_deps.len());

            for dep in conda_deps {
                match dep {
                    CondaDependency::Simple(s) | CondaDependency::Versioned(s) => {
                        let (name, version) = self.parse_conda_dependency(&s);

                        // Skip non-Python packages
                        if self.should_skip_package(&name) {
                            debug!("Skipping non-Python package: {}", name);
                            continue;
                        }

                        // Map Conda package name to PyPI equivalent
                        let pypi_name = self.map_conda_to_pypi_name(&name);

                        // Update problematic versions
                        let updated_version = self.update_problematic_versions(&pypi_name, version);

                        dependencies.push(Dependency {
                            name: pypi_name,
                            version: updated_version,
                            dep_type: DependencyType::Main,
                            environment_markers: None,
                            extras: None,
                        });
                    }
                    CondaDependency::Pip(pip_map) => {
                        // Process pip dependencies
                        if let Some(pip_deps) = pip_map.get("pip") {
                            debug!("Processing {} pip dependencies", pip_deps.len());
                            let mut pip_dependencies = self.process_pip_dependencies(pip_deps);
                            dependencies.append(&mut pip_dependencies);
                        }
                    }
                }
            }
        }

        info!(
            "Extracted {} dependencies from Conda environment",
            dependencies.len()
        );
        Ok(dependencies)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_environment(content: &str) -> (TempDir, std::path::PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        fs::write(project_dir.join("environment.yml"), content).unwrap();

        (temp_dir, project_dir)
    }

    #[test]
    fn test_detect_conda_project() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        // Test with environment.yml
        fs::write(project_dir.join("environment.yml"), "").unwrap();
        assert!(CondaMigrationSource::detect_project_type(&project_dir));

        // Clean up and test with environment.yaml
        fs::remove_file(project_dir.join("environment.yml")).unwrap();
        fs::write(project_dir.join("environment.yaml"), "").unwrap();
        assert!(CondaMigrationSource::detect_project_type(&project_dir));

        // Test without environment file
        fs::remove_file(project_dir.join("environment.yaml")).unwrap();
        assert!(!CondaMigrationSource::detect_project_type(&project_dir));
    }

    #[test]
    fn test_parse_conda_dependency() {
        let source = CondaMigrationSource;

        // Test simple package
        assert_eq!(
            source.parse_conda_dependency("numpy"),
            ("numpy".to_string(), None)
        );

        // Test versioned package
        assert_eq!(
            source.parse_conda_dependency("pandas=1.3.0"),
            ("pandas".to_string(), Some("==1.3.0".to_string()))
        );

        // Test package with comparison
        assert_eq!(
            source.parse_conda_dependency("scipy>=1.7"),
            ("scipy".to_string(), Some(">=1.7".to_string()))
        );
    }

    #[test]
    fn test_convert_wildcard_version() {
        let source = CondaMigrationSource;

        // Test major wildcard
        assert_eq!(
            source.convert_wildcard_version("1.*"),
            Some(">=1.0.0,<2.0.0".to_string())
        );

        // Test minor wildcard
        assert_eq!(
            source.convert_wildcard_version("1.2.*"),
            Some(">=1.2.0,<1.3.0".to_string())
        );

        // Test any version
        assert_eq!(source.convert_wildcard_version("*"), None);
    }

    #[test]
    fn test_extract_dependencies() {
        let content = r#"
name: test-env
channels:
  - conda-forge
  - defaults
dependencies:
  - python=3.9
  - numpy=1.21.*
  - pandas>=1.3.0
  - scikit-learn
  - pip
  - pip:
    - requests==2.28.0
    - flask[async]>=2.0.0
"#;

        let (_temp_dir, project_dir) = create_test_environment(content);
        let source = CondaMigrationSource;
        let dependencies = source.extract_dependencies(&project_dir).unwrap();

        // Should skip python and pip
        assert_eq!(dependencies.len(), 5);

        // Check numpy
        let numpy_dep = dependencies.iter().find(|d| d.name == "numpy").unwrap();
        assert_eq!(numpy_dep.version, Some(">=1.21.0,<1.22.0".to_string()));

        // Check pandas
        let pandas_dep = dependencies.iter().find(|d| d.name == "pandas").unwrap();
        assert_eq!(pandas_dep.version, Some(">=1.3.0".to_string()));

        // Check scikit-learn
        let sklearn_dep = dependencies
            .iter()
            .find(|d| d.name == "scikit-learn")
            .unwrap();
        assert_eq!(sklearn_dep.version, None);

        // Check requests (from pip)
        let requests_dep = dependencies.iter().find(|d| d.name == "requests").unwrap();
        assert_eq!(requests_dep.version, Some("==2.28.0".to_string()));

        // Check flask with extras
        let flask_dep = dependencies.iter().find(|d| d.name == "flask").unwrap();
        assert_eq!(flask_dep.version, Some(">=2.0.0".to_string()));
        assert_eq!(flask_dep.extras, Some(vec!["async".to_string()]));
    }

    #[test]
    fn test_package_name_mapping() {
        let source = CondaMigrationSource;

        assert_eq!(source.map_conda_to_pypi_name("pytorch"), "torch");
        assert_eq!(source.map_conda_to_pypi_name("py-opencv"), "opencv-python");
        assert_eq!(source.map_conda_to_pypi_name("numpy"), "numpy"); // No mapping needed
    }
}
