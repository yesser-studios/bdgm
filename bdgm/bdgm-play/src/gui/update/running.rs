use std::sync::Arc;

use bdgm::game::ValidatedGame;
use iced::Task;

use crate::core::{AppError, LaunchHandle};

use super::super::{
    message::Message,
    state::{AppState, Status},
};

/// Stores the launched game and handle as the current playing state.
pub(super) fn on_game_started(
    state: &mut AppState,
    game: ValidatedGame,
    handle: LaunchHandle,
) -> Task<Message> {
    state.status = Status::Playing(game, handle);
    Task::none()
}

/// Attempts to reopen the playing HTML game in a browser, ignoring opening failures.
pub(super) fn on_open_html_game_clicked(state: &AppState) -> Task<Message> {
    if let Status::Playing(_, LaunchHandle::HtmlServer(server)) = &state.status {
        server.open();
    }

    Task::none()
}

/// Returns a termination task for the playing game, or no task when no game is playing.
pub(super) fn on_stop_game_clicked(state: &mut AppState) -> Task<Message> {
    let handle = match &state.status {
        Status::Playing(_, handle) => handle.clone(),
        _ => return Task::none(),
    };

    Task::perform(async move { handle.kill().await }, |_| Message::None)
}

/// Stores a completed game and optional exit code, or shows a wait error in the error state.
pub(super) fn on_game_exited(
    state: &mut AppState,
    game: ValidatedGame,
    exit_code: crate::core::Result<Option<i32>>,
) -> Task<Message> {
    match exit_code {
        Ok(exit_code) => {
            state.status = Status::Idle(Some((game, exit_code)));
            Task::none()
        }
        Err(error) => on_error(state, error),
    }
}

/// Replaces the current status with an error for display.
pub(super) fn on_error(state: &mut AppState, error: AppError) -> Task<Message> {
    state.status = Status::Error(Arc::new(error));
    Task::none()
}
