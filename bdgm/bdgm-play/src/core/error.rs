use std::{io, path::PathBuf};

use bdgm::error::BDGMErrors;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Failed to parse app directories")]
    NoAppDirs,
    #[error(transparent)]
    ExtractError(#[from] ExtractError),
    #[error(transparent)]
    ValidationError(#[from] BDGMErrors),
    #[error(transparent)]
    InstallError(#[from] InstallError),
    #[error(transparent)]
    LaunchError(#[from] LaunchError),
    #[error(transparent)]
    RunningError(#[from] RunningError),
    #[error("{0} is not supported on this platform.")]
    UnsupportedOnThisPlatform(UnsupportedOperation),
    #[error("{0} is not supported.")]
    UnsupportedOperation(UnsupportedOperation),
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
    #[error("Runtime {} was not found, is it installed?", .0.display())]
    RuntimeNotFound(PathBuf),
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error("Port {0} is already in use")]
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
