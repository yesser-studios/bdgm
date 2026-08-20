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
