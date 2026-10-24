use std::{io, path::PathBuf};

use bdgm::error::BDGMErrors;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Failed to parse app directories")]
    NoAppDirs,
    #[error("An error occurred during extraction: {0}")]
    ExtractError(#[from] ExtractError),
    #[error("An error occurred during validation: {0}")]
    ValidationError(#[from] BDGMErrors),
    #[error("An error occurred during installation: {0}")]
    InstallError(#[from] InstallError),
    #[error("An error occurred during launch: {0}")]
    LaunchError(#[from] LaunchError),
    #[error("An error occurred when running: {0}")]
    RunningError(#[from] RunningError),
    #[error("{0} is not supported on this platform.")]
    UnsupportedOnThisPlatform(UnsupportedOperation),
    #[error("{0} is not supported.")]
    UnsupportedOperation(UnsupportedOperation),
    #[error("An IO error occurred: {0}")]
    IoError(#[from] io::Error),
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Error)]
pub enum ExtractError {
    #[error(transparent)]
    IoError(#[from] io::Error),
}

#[derive(Debug, Error)]
pub enum InstallError {
    #[error(transparent)]
    IoError(#[from] io::Error),
}

#[derive(Debug, Error)]
pub enum LaunchError {
    #[error("Runtime {}{} (path: `{}`) was not found, is it installed?",
       .0.runtime().display_name(),
       .0.runtime_version().map(|v| format!(" {v}")).unwrap_or_default(),
       .1.display()
    )]
    RuntimeNotFound(Box<bdgm::game::ValidatedGame>, PathBuf),
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error("Port {0} is already in use.")]
    PortTaken(u16),
}

#[derive(Debug, Error)]
pub enum RunningError {
    #[error(transparent)]
    IoError(#[from] io::Error),
}

#[derive(Debug, Error)]
pub enum UnsupportedOperation {
    #[error("Reading raw discs")]
    ReadingRawDiscs,
}
