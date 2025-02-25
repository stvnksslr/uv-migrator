use std::fmt;
use std::io;
use std::path::PathBuf;

/// Custom error type for UV Migrator operations
#[derive(Debug)]
pub enum Error {
    /// I/O errors (file access, permissions, etc.)
    Io(io::Error),

    /// TOML parsing errors
    Toml(toml_edit::TomlError),

    /// TOML serialization/deserialization errors
    TomlSerde(toml::de::Error),

    /// Errors from UV command execution
    UvCommand(String),

    /// Errors related to project detection
    ProjectDetection(String),

    /// Errors related to dependency parsing
    DependencyParsing(String),

    /// Errors related to file operations
    FileOperation { path: PathBuf, message: String },

    /// General errors
    General(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(err) => write!(f, "I/O error: {}", err),
            Error::Toml(err) => write!(f, "TOML parsing error: {}", err),
            Error::TomlSerde(err) => write!(f, "TOML serialization error: {}", err),
            Error::UvCommand(msg) => write!(f, "UV command failed: {}", msg),
            Error::ProjectDetection(msg) => write!(f, "Project detection error: {}", msg),
            Error::DependencyParsing(msg) => write!(f, "Dependency parsing error: {}", msg),
            Error::FileOperation { path, message } => {
                write!(f, "File operation error on {}: {}", path.display(), message)
            }
            Error::General(msg) => write!(f, "{}", msg),
        }
    }
}

impl Error {
    /// Check if the error message contains a specific string
    #[allow(dead_code)]
    pub fn contains(&self, needle: &str) -> bool {
        match self {
            Error::FileOperation { path: _, message } => message.contains(needle),
            _ => {
                let message = self.to_string();
                message.contains(needle)
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            Error::Toml(err) => Some(err),
            Error::TomlSerde(err) => Some(err),
            _ => None,
        }
    }
}

// Implement From conversions for common error types
impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<toml_edit::TomlError> for Error {
    fn from(err: toml_edit::TomlError) -> Self {
        Error::Toml(err)
    }
}

impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Self {
        Error::TomlSerde(err)
    }
}

impl From<String> for Error {
    fn from(err: String) -> Self {
        Error::General(err)
    }
}

impl From<&str> for Error {
    fn from(err: &str) -> Self {
        Error::General(err.to_string())
    }
}

/// Result type alias for UV Migrator operations
pub type Result<T> = std::result::Result<T, Error>;
