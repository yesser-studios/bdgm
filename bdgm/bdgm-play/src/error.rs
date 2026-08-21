use std::fmt::Display;

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum AppError {
    NoAppDirs,
}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::NoAppDirs => write!(f, "Failed to parse app directories"),
        }
    }
}
