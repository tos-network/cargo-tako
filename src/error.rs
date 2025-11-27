//! Error types for cargo-tako

use std::fmt;
use std::io;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    ProjectExists(String),
    InvalidTemplate(String),
    BuildFailed(String),
    TestFailed(String),
    Config(String),
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(err) => write!(f, "IO error: {err}"),
            Error::ProjectExists(name) => write!(f, "Project '{name}' already exists"),
            Error::InvalidTemplate(name) => write!(f, "Invalid template: {name}"),
            Error::BuildFailed(msg) => write!(f, "Build failed: {msg}"),
            Error::TestFailed(msg) => write!(f, "Tests failed: {msg}"),
            Error::Config(msg) => write!(f, "Configuration error: {msg}"),
            Error::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Self {
        Error::Config(err.to_string())
    }
}

impl From<toml::ser::Error> for Error {
    fn from(err: toml::ser::Error) -> Self {
        Error::Config(err.to_string())
    }
}
