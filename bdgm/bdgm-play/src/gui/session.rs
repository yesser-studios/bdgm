use std::sync::{Arc, Mutex};

use iced::Task;

use crate::cli::args::Args;

use super::{
    message::Message,
    state::{AppState, Slot, StartedSession},
    tasks::start_game_task,
};

pub(crate) fn is_busy(state: &AppState) -> bool {
    matches!(
        state.phase,
        super::state::Phase::Starting | super::state::Phase::Playing
    )
}

/// Shared busy guard: reports "already running" and returns `true` when the
/// caller should bail out with `Task::none()`.
pub(crate) fn ensure_not_busy(state: &mut AppState) -> bool {
    if is_busy(state) {
        state.status = String::from("A game is already running — stop it first.");
        true
    } else {
        false
    }
}

pub(crate) fn begin_launch(state: &mut AppState, args: Args) -> Task<Message> {
    if ensure_not_busy(state) {
        return Task::none();
    }
    let slot: Slot = Arc::new(Mutex::new(None));
    state.starting_slot = Some(Arc::clone(&slot));
    state.phase = super::state::Phase::Starting;
    state.status = String::from("Starting game...");
    Task::perform(start_game_task(args, slot), Message::GameStarted)
}

pub(crate) fn take_starting_session(state: &mut AppState) -> Option<StartedSession> {
    state.starting_slot.take()?.lock().ok()?.take()
}
