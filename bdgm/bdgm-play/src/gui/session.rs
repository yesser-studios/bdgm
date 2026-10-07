use std::sync::{Arc, Mutex};

use iced::Task;

use crate::cli::args::Args;

use super::{
    message::Message,
    state::{AppState, Slot, StartedSession},
    tasks::start_game_stream,
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
    state.dump_progress = None;
    Task::stream(start_game_stream(args, slot))
}

pub(crate) fn take_starting_session(state: &mut AppState) -> Option<StartedSession> {
    state.starting_slot.take()?.lock().ok()?.take()
}
