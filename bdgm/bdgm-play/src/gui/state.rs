use std::{path::PathBuf, sync::Arc};

use bdgm::game::ValidatedGame;

use crate::core::{AppError, LaunchHandle, State};

pub(super) enum Status {
    Idle(Option<(ValidatedGame, Option<i32>)>),
    Starting(Arc<State>),
    Playing(ValidatedGame, LaunchHandle),
    Error(Arc<AppError>),
}

pub(super) struct AppState {
    pub(super) runtime: Option<PathBuf>,
    pub(super) status: Status,
    pub(super) disc_picker_discs: Option<Vec<char>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            runtime: None,
            status: Status::Idle(None),
            disc_picker_discs: None,
        }
    }
}

impl Drop for AppState {
    fn drop(&mut self) {
        // HTML servers do not outlive the window; game processes are allowed
        // to keep running after it closes.
        if let Status::Playing(_, LaunchHandle::HtmlServer(server)) = &self.status {
            server.kill();
        }
    }
}
