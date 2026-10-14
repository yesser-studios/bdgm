use iced::Task;

use super::super::{
    message::Message,
    state::{AppState, Phase},
};

pub(super) fn handle_game_exited(
    state: &mut AppState,
    result: Result<Option<i32>, String>,
) -> Task<Message> {
    // Natural exit or server crash: auto-close the modal by leaving
    // `Playing`. Dropping `playing_tempdir` cleans the extract dir;
    // the port was already freed because the server task ended.
    reset_playing_state(state);
    match result {
        Ok(code) => handle_game_exited_ok(state, code),
        Err(err) => handle_game_exited_err(state, err),
    }
    Task::none()
}

fn reset_playing_state(state: &mut AppState) {
    state.playing_handle = None;
    state.playing_tempdir = None;
    state.starting_slot = None;
    state.phase = Phase::Idle;
}

fn handle_game_exited_ok(state: &mut AppState, code: Option<i32>) {
    let name = state
        .playing_info
        .as_ref()
        .map(|i| i.name.clone())
        .unwrap_or_else(|| String::from("Game"));
    state.status = match code {
        Some(0) | None => format!("{name} exited."),
        Some(c) => format!("{name} exited with code {c}."),
    };
}

fn handle_game_exited_err(state: &mut AppState, err: String) {
    state.status = format!("Game ended: {err}");
}
