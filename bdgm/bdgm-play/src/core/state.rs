use std::{fmt::Display, path::PathBuf};

use bdgm::game::ValidatedGame;

use super::error::AppError;

pub trait StateHandler: Send + Sync {
    fn change_state(&mut self, state: State);

    fn revert_state(&mut self);
}

#[derive(Debug)]
pub enum State {
    StartingExtraction,
    Extracting(Option<(u64, u64)>),
    Validating,
    Installing(ValidatedGame),
    Launching(ValidatedGame),
    WaitingForLock(PathBuf),
    Error(AppError),
}

impl Display for State {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::StartingExtraction => write!(formatter, "Preparing extraction..."),
            State::Extracting(None) => write!(formatter, "Extracting..."),
            State::Extracting(Some((current, total))) => {
                write!(formatter, "Extracting... {current}/{total} bytes")
            }
            State::Validating => write!(formatter, "Validating game..."),
            State::Installing(game) => write!(
                formatter,
                "Installing {} version {}...",
                game.name(),
                game.version()
            ),
            State::Launching(game) => write!(formatter, "Launching {}...", game.name()),
            State::WaitingForLock(path) => {
                write!(formatter, "Waiting for lock on {}...", path.display())
            }
            State::Error(error) => write!(formatter, "{error}"),
        }
    }
}
