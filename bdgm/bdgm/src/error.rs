use std::fmt::Display;
use thiserror::Error;

use crate::runtime::Runtime;

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

/// Represents a single BDGM validation error.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BDGMError {
    UnableToParse(ParserError),
    MissingHeaderVersion,
    UnsupportedHeaderVersion(String),
    BDGMDirectoryMissing,
    DiscFileMissing,
    DiscFileInvalid,
    MandatoryFieldMissing(String),
    IdFormatInvalid(IdFormatError),
    RuntimeInvalid(String),
    ExecutablePathForbidden(ExecutableError),
    AppDirectoryMissing,
    ExecutableMissing(String),
    ArgsInvalid(String),
    RuntimeArgsInvalid(String),
}

impl Display for BDGMError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BDGMError::UnableToParse(e) => write!(f, "Unable to parse manifest: {e}"),
            BDGMError::MissingHeaderVersion => write!(f, "The BDGM header is missing!"),
            BDGMError::UnsupportedHeaderVersion(version) => {
                write!(f, "BDGM spec version {version} is not supported")
            }
            BDGMError::BDGMDirectoryMissing => write!(f, "The BDGM directory is missing!"),
            BDGMError::DiscFileMissing => write!(f, "The DISC.BDGM file is missing!"),
            BDGMError::DiscFileInvalid => write!(f, "The DISC.BDGM file is invalid!"),
            BDGMError::AppDirectoryMissing => write!(f, "The APP directory is missing!"),
            BDGMError::ExecutableMissing(path) => {
                write!(f, "The executable BDGM/APP/{path} is missing!")
            }
            BDGMError::MandatoryFieldMissing(field) => {
                write!(f, "The mandatory field {field} is missing!")
            }
            BDGMError::IdFormatInvalid(reason) => {
                write!(f, "The format of the ID is invalid, because: {reason}")
            }
            BDGMError::RuntimeInvalid(runtime) => write!(
                f,
                "The runtime {runtime} is invalid. Valid runtimes: {}",
                Runtime::display_all()
            ),
            BDGMError::ExecutablePathForbidden(reason) => {
                write!(f, "The path to the executable is forbidden: {reason}")
            }
            BDGMError::ArgsInvalid(reason) => {
                write!(
                    f,
                    "The value of the args field could not be parsed: {reason}"
                )
            }
            BDGMError::RuntimeArgsInvalid(reason) => {
                write!(
                    f,
                    "The value of the runtime_args field could not be parsed: {reason}"
                )
            }
        }
    }
}

/// A set of BDGM validation errors.
#[derive(Debug, Error, PartialEq, Eq, Default)]
pub struct BDGMErrors(pub Vec<BDGMError>);

impl BDGMErrors {
    /// Adds a `BDGMError` to the current list.
    pub fn add(&mut self, error: BDGMError) {
        self.0.push(error);
    }
    /// Returns `Ok` if the set contains no errors, otherwise returns `Err(self)`.
    pub fn evaluate(self) -> Result<(), Self> {
        if !self.0.is_empty() {
            Err(self)
        } else {
            Ok(())
        }
    }
}

impl From<BDGMError> for BDGMErrors {
    fn from(error: BDGMError) -> Self {
        Self(vec![error])
    }
}

impl Display for BDGMErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Errors:")?;
        for err in self.0.iter() {
            write!(f, "\n{err}")?;
        }
        Ok(())
    }
}

/// Represents a single parser error.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParserError {
    WhitespaceInKey(String),
    LeadingWhitespace(String),
    TrailingWhitespace(String),
    EmptyValue(String),
    InvalidHeader,
    InvalidRuntime,
}

impl Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserError::WhitespaceInKey(key) => write!(f, "There is whitespace in key: {key}"),
            ParserError::LeadingWhitespace(line) => {
                write!(f, "There is a leading whitespace in line: {line}")
            }
            ParserError::TrailingWhitespace(line) => {
                write!(f, "There is a trailing whitespace in line: {line}")
            }
            ParserError::EmptyValue(key) => write!(f, "{key} has an empty value!"),
            ParserError::InvalidHeader => write!(f, "Invalid header!"),
            ParserError::InvalidRuntime => write!(f, "Invalid runtime value!"),
        }
    }
}

impl From<ParserError> for BDGMError {
    fn from(value: ParserError) -> Self {
        BDGMError::UnableToParse(value)
    }
}

/// Represents a single ID format validation error.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum IdFormatError {
    ContainsUppercase,
    ContainsUnderscores,
    ContainsWhitespace,
    ContainsForbiddenCharacter(char),
    IncorrectStructure,
}

impl Display for IdFormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdFormatError::ContainsUppercase => {
                write!(
                    f,
                    "The ID contains uppercase characters. Only lowercase characters may be used."
                )
            }
            IdFormatError::ContainsUnderscores => {
                write!(
                    f,
                    "The ID contains underscores. Use dashes to separate words instead."
                )
            }
            IdFormatError::ContainsWhitespace => write!(
                f,
                "The ID contains whitespace characters. Use dashes to separate words instead."
            ),
            IdFormatError::ContainsForbiddenCharacter(character) => {
                write!(f, "The ID contains a forbidden character: {character}")
            }
            IdFormatError::IncorrectStructure => {
                write!(
                    f,
                    "The ID doesn't follow the correct structure. Use `<your company domain reversed>.<your game name>`. For example: com.example.example-game, io.github.yesser-studios.pong, net.bethesda.doom"
                )
            }
        }
    }
}

impl From<IdFormatError> for BDGMError {
    fn from(value: IdFormatError) -> Self {
        BDGMError::IdFormatInvalid(value)
    }
}

/// Represents a single executable validation error.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExecutableError {
    #[error("The path is absolute. Only paths relative to DISC.BDGM can be used.")]
    IsAbsolute,
    #[error("The path contains the forbidden `.` element.")]
    ContainsCurrentDir,
    #[error("The path contains the forbidden `..` element.")]
    ContainsParentDir,
    #[error("The path contains a backslash. Use slashes to separate path components.")]
    BackslashSeparated,
}

impl From<ExecutableError> for BDGMError {
    fn from(value: ExecutableError) -> Self {
        BDGMError::ExecutablePathForbidden(value)
    }
}
