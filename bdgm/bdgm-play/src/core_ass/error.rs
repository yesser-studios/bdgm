use std::{fmt::Display, fs, io, path::Path};

use bdgm::error::BDGMError;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AppError {
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

#[derive(Debug, Error)]
pub enum IoError {
    IoError(io::Error),
    TryLockError(fs::TryLockError),
    FsExtraError(fs_extra::error::Error),
    PathNone,
}

impl Display for IoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IoError::IoError(error) => error.fmt(f),
            IoError::TryLockError(error) => error.fmt(f),
            IoError::FsExtraError(error) => error.fmt(f),
            IoError::PathNone => write!(f, "Path is None"),
        }
    }
}

impl From<io::Error> for IoError {
    fn from(value: io::Error) -> Self {
        Self::IoError(value)
    }
}
impl From<fs::TryLockError> for IoError {
    fn from(value: fs::TryLockError) -> Self {
        Self::TryLockError(value)
    }
}
impl From<fs_extra::error::Error> for IoError {
    fn from(value: fs_extra::error::Error) -> Self {
        Self::FsExtraError(value)
    }
}

/// Error for a missing runtime binary when spawning a game process.
///
/// Displays as `Runtime <display name>[ <version>] was not found, is it installed?`.
#[derive(Debug, Error)]
pub struct RuntimeNotFound {
    display: String,
    version: Option<String>,
}

impl Display for RuntimeNotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.version {
            Some(version) => write!(
                f,
                "Runtime {} {} was not found, is it installed?",
                self.display, version
            ),
            None => write!(
                f,
                "Runtime {} was not found, is it installed?",
                self.display
            ),
        }
    }
}

impl RuntimeNotFound {
    pub fn from_game(game: &bdgm::game::ValidatedGame) -> Self {
        Self {
            display: game.runtime().display_name().to_string(),
            version: game.runtime_version().map(|s| s.to_owned()),
        }
    }

    pub fn from_path(path: &Path, version: Option<String>) -> Self {
        Self {
            display: path.display().to_string(),
            version,
        }
    }
}
