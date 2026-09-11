use std::fmt::Display;

use bdgm::error::BDGMError;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum AppError {
    NoAppDirs,
    InvalidGameFile(BDGMError),
    CouldNotFindUnclaimedPort,
}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::NoAppDirs => write!(f, "Failed to parse app directories"),
            AppError::InvalidGameFile(e) => write!(f, "The disc manifest is invalid: {e}"),
            AppError::CouldNotFindUnclaimedPort => {
                write!(f, "Could not find a port that hasn't been claimed yet")
            }
        }
    }
}
