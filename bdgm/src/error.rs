use std::fmt::Display;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParserError {
    InvalidHeader,
    InvalidRuntime,
}

impl Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserError::InvalidHeader => write!(f, "Invalid header!"),
            ParserError::InvalidRuntime => write!(f, "Invalid runtime value!"),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EntryError {
    EntryIsFile,
    EntryIsDirectory,
}

impl Display for EntryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntryError::EntryIsFile => write!(f, "The entry is a file!"),
            EntryError::EntryIsDirectory => write!(f, "The entry is a directory!"),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BDGMError {
    BDGMDirectoryMissing,
    DiscFileMissing,
    DiscFileInvalid,
    AppDirectoryMissing,
    ExecutableMissing,
}

impl Display for BDGMError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BDGMError::BDGMDirectoryMissing => write!(f, "The BDGM directory is missing!"),
            BDGMError::DiscFileMissing => write!(f, "The DISC.BDGM file is missing!"),
            BDGMError::DiscFileInvalid => write!(f, "The DISC.BDGM file is invalid!"),
            BDGMError::AppDirectoryMissing => write!(f, "The APP directory is missing!"),
            BDGMError::ExecutableMissing => {
                write!(f, "The executable specified in DISC.BDGM is missing!")
            }
        }
    }
}
