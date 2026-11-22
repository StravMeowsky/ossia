use std::{fmt, io};

pub mod download;
pub mod feed;
pub mod fetch;
pub mod github;
pub mod install;
pub mod notes;
pub mod ossia;
pub mod runtime;
pub mod schedule;
pub mod service;
pub mod stage;
pub mod target;
pub mod verify;
pub mod version;

pub use service::{InstallOutcome, UpdateService};

#[derive(Debug)]
pub struct Error(String);

impl Error {
    pub fn other(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self(error.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self(error.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
#[path = "../../tests/update/mod.rs"]
mod tests;
