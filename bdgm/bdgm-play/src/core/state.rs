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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::core::test_utils::{manifest, validated_game};

    #[test]
    fn states_display_progress_messages() {
        assert_eq!(
            State::StartingExtraction.to_string(),
            "Preparing extraction..."
        );
        assert_eq!(State::Extracting(None).to_string(), "Extracting...");
        assert_eq!(
            State::Extracting(Some((10, 20))).to_string(),
            "Extracting... 10/20 bytes"
        );
        assert_eq!(State::Validating.to_string(), "Validating game...");

        let game = validated_game(&manifest(
            "com.example.test",
            "1.0.0",
            "windows",
            "game.exe",
            "[]",
            "[]",
        ));
        assert_eq!(
            State::Installing(game.clone()).to_string(),
            "Installing Test Game version 1.0.0..."
        );
        assert_eq!(State::Launching(game).to_string(), "Launching Test Game...");
        assert_eq!(
            State::WaitingForLock(PathBuf::from("game.lock")).to_string(),
            "Waiting for lock on game.lock..."
        );
        assert_eq!(
            State::Error(AppError::Other("oops".to_string())).to_string(),
            "oops"
        );
    }
}
