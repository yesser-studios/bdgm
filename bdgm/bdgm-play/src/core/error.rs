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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_error_messages_are_human_readable() {
        assert_eq!(
            AppError::NoAppDirs.to_string(),
            "Failed to parse app directories"
        );
        assert_eq!(
            AppError::LaunchError(LaunchError::RuntimeNotFound(PathBuf::from("java"))).to_string(),
            "Runtime java was not found, is it installed?"
        );
        assert_eq!(
            AppError::LaunchError(LaunchError::PortTaken(1234)).to_string(),
            "Port 1234 is already in use"
        );
        assert_eq!(
            AppError::UnsupportedOnThisPlatform(UnsupportedOperation::ReadingRawDiscs).to_string(),
            "Reading raw discs is not supported on this platform."
        );
        assert_eq!(
            AppError::UnsupportedOperation(UnsupportedOperation::ReadingRawDiscs).to_string(),
            "Reading raw discs is not supported."
        );
        assert_eq!(AppError::Other("oops".to_string()).to_string(), "oops");
    }

    #[test]
    fn wrapped_errors_keep_their_messages() {
        let io_error = || io::Error::other("underlying error");

        assert_eq!(
            AppError::from(ExtractError::IoError(io_error())).to_string(),
            "underlying error"
        );
        assert_eq!(
            AppError::from(InstallError::IoError(io_error())).to_string(),
            "underlying error"
        );
        assert_eq!(
            AppError::from(LaunchError::IoError(io_error())).to_string(),
            "underlying error"
        );
        assert_eq!(
            AppError::from(RunningError::IoError(io_error())).to_string(),
            "underlying error"
        );
    }
}
