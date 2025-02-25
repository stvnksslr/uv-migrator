/// Represents a project dependency with its type, version, and other requirements
#[derive(Debug, Clone)]
pub struct Dependency {
    /// The name of the dependency package
    pub name: String,

    /// Optional version constraint
    pub version: Option<String>,

    /// Type of the dependency (main, dev, or specific group)
    pub dep_type: DependencyType,

    /// Optional environment markers (e.g. "python_version > '3.7'")
    pub environment_markers: Option<String>,
}

/// Represents the type of dependency
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DependencyType {
    /// Main project dependency
    Main,

    /// Development dependency
    Dev,

    /// Dependency in a specific group (e.g. "docs", "test")
    Group(String),
}

use std::str::FromStr;

impl FromStr for DependencyType {
    type Err = String;

    /// Converts a string representation to a DependencyType
    fn from_str(dep_type: &str) -> Result<Self, Self::Err> {
        Ok(match dep_type {
            "dev" => DependencyType::Dev,
            "main" => DependencyType::Main,
            group => DependencyType::Group(group.to_string()),
        })
    }
}

impl DependencyType {
    /// Converts a string representation to a DependencyType without error handling
    #[allow(dead_code)]
    pub fn parse_str(dep_type: &str) -> Self {
        match dep_type {
            "dev" => DependencyType::Dev,
            "main" => DependencyType::Main,
            group => DependencyType::Group(group.to_string()),
        }
    }
}

impl Dependency {
    /// Creates a new dependency with the given name and dependency type
    #[allow(dead_code)]
    pub fn new(name: String, dep_type: DependencyType) -> Self {
        Self {
            name,
            version: None,
            dep_type,
            environment_markers: None,
        }
    }

    /// Creates a new dependency with the given name, version, and dependency type
    #[allow(dead_code)]
    pub fn with_version(name: String, version: String, dep_type: DependencyType) -> Self {
        Self {
            name,
            version: Some(version),
            dep_type,
            environment_markers: None,
        }
    }

    /// Adds environment markers to the dependency
    #[allow(dead_code)]
    pub fn with_markers(mut self, markers: String) -> Self {
        self.environment_markers = Some(markers);
        self
    }
}
