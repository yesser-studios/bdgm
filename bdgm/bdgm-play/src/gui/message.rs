use std::{path::PathBuf, sync::Arc};

use bdgm::game::ValidatedGame;

use crate::core::{AppError, LaunchHandle, State};

pub(super) enum Message {
    OpenDiscClicked,
    OpenImageClicked,
    DiscPickerCancel,
    // Only constructed by the raw disc picker, which exists on Windows.
    #[allow(dead_code)]
    RunRawDisc(char),
    RunMounted(PathBuf),
    RunImage(PathBuf),
    StateChanged(Arc<State>),
    GameStarted(ValidatedGame, LaunchHandle),
    StopGameClicked,
    OpenHTMLGameClicked,
    GameExited(ValidatedGame, crate::core::Result<Option<i32>>),
    GameError(AppError),
    None,
}

// iced requires messages to be cloneable. `AppError` is not `Clone`, but
// cloned errors are only used as terminal display text, so preserve their
// message without preserving their concrete variant.
impl Clone for Message {
    fn clone(&self) -> Self {
        match self {
            Self::OpenDiscClicked => Self::OpenDiscClicked,
            Self::OpenImageClicked => Self::OpenImageClicked,
            Self::DiscPickerCancel => Self::DiscPickerCancel,
            Self::RunRawDisc(disc) => Self::RunRawDisc(*disc),
            Self::RunMounted(path) => Self::RunMounted(path.clone()),
            Self::RunImage(path) => Self::RunImage(path.clone()),
            Self::StateChanged(state) => Self::StateChanged(Arc::clone(state)),
            Self::GameStarted(game, handle) => Self::GameStarted(game.clone(), handle.clone()),
            Self::StopGameClicked => Self::StopGameClicked,
            Self::OpenHTMLGameClicked => Self::OpenHTMLGameClicked,
            Self::GameExited(game, result) => {
                Self::GameExited(game.clone(), result.as_ref().map_err(clone_error).copied())
            }
            Self::GameError(error) => Self::GameError(clone_error(error)),
            Self::None => Self::None,
        }
    }
}

fn clone_error(error: &AppError) -> AppError {
    AppError::Other(error.to_string())
}
