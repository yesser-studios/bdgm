use std::{fmt::Display, path::PathBuf};

use bdgm::game::ValidatedGame;

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
}

impl Display for State {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::StartingExtraction => write!(formatter, "Preparing extraction..."),
            State::Extracting(None) => write!(formatter, "Extracting..."),
            State::Extracting(Some((current, total))) => {
                write!(
                    formatter,
                    "Extracting... {:.2} / {:.2} MiB",
                    *current as f64 / 1024.0 / 1024.0,
                    *total as f64 / 1024.0 / 1024.0
                )
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
            "Extracting... 0.00 / 0.00 MiB"
        );
        assert_eq!(
            State::Extracting(Some((1024 * 1024, 2 * 1024 * 1024))).to_string(),
            "Extracting... 1.00 / 2.00 MiB"
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
    }
}
