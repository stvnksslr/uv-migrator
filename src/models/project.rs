use serde::Deserialize;
use std::collections::HashMap;

/// Represents a detected project type
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectType {
    /// Poetry project (application or package)
    Poetry(PoetryProjectType),
    /// Pipenv project
    Pipenv,
    /// Requirements.txt based project
    Requirements,
    /// Setup.py based project
    SetupPy,
}

/// Distinguishes between Poetry application and package projects
#[derive(Debug, Clone, PartialEq)]
pub enum PoetryProjectType {
    /// Poetry application project (no packages section)
    Application,
    /// Poetry package project (has packages section)
    Package,
}

/// Represents the entire pyproject.toml file structure
#[derive(Deserialize, Debug)]
#[allow(dead_code)] // Fields used through Serde deserialization
pub struct PyProject {
    pub tool: Option<Tool>,
    pub project: Option<Project>,
}

/// Represents the [tool] section of pyproject.toml
#[derive(Deserialize, Debug)]
#[allow(dead_code)] // Fields used through Serde deserialization
pub struct Tool {
    pub poetry: Option<Poetry>,
}

/// Represents the top-level [project] section (Poetry 2.0 style)
#[derive(Deserialize, Debug)]
#[allow(dead_code)] // Fields used through Serde deserialization
pub struct Project {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Option<Vec<AuthorConfig>>,
    pub readme: Option<String>,
    pub requires_python: Option<String>,
    pub dependencies: Option<Vec<String>>,
}

/// Represents an author configuration
#[derive(Deserialize, Debug)]
#[allow(dead_code)] // Fields used through Serde deserialization
pub struct AuthorConfig {
    pub name: String,
    pub email: Option<String>,
}

/// Represents the [tool.poetry] section
#[derive(Deserialize, Debug)]
#[allow(dead_code)] // Fields used through Serde deserialization
pub struct Poetry {
    pub dependencies: Option<HashMap<String, toml::Value>>,
    pub group: Option<HashMap<String, Group>>,
    pub packages: Option<Vec<Package>>,
}

/// Represents a package configuration in [tool.poetry.packages]
#[derive(Deserialize, Debug)]
#[allow(dead_code)] // Fields used through Serde deserialization
pub struct Package {
    pub include: Option<String>,
}

/// Represents a dependency group in [tool.poetry.group]
#[derive(Deserialize, Debug)]
#[allow(dead_code)] // Fields used through Serde deserialization
pub struct Group {
    pub dependencies: HashMap<String, toml::Value>,
}
