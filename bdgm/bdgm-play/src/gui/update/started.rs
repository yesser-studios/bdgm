use iced::Task;
use tempfile::TempDir;

use super::super::{
    message::Message,
    session::take_starting_session,
    state::{AppState, GameInfo, Phase, PlayHandle, StartedHandle},
    tasks::{wait_process_task, wait_server_task},
};

pub(super) fn handle_game_started(
    state: &mut AppState,
    result: Result<GameInfo, String>,
) -> Task<Message> {
    match result {
        Ok(_) => handle_game_started_ok(state),
        Err(err) => handle_game_started_err(state, err),
    }
}

fn handle_game_started_ok(state: &mut AppState) -> Task<Message> {
    state.dump_progress = None;
    let Some(session) = take_starting_session(state) else {
        state.phase = Phase::Idle;
        state.status = String::from("Game started but session was lost.");
        return Task::none();
    };
    state.status = format!("Running: {}", session.info.name);
    state.playing_info = Some(session.info.clone());
    match session.handle {
        StartedHandle::Process(proc) => attach_process_session(state, proc, session.tempdir),
        StartedHandle::Server { join, abort } => {
            attach_server_session(state, join, abort, session.tempdir)
        }
    }
}

fn handle_game_started_err(state: &mut AppState, err: String) -> Task<Message> {
    state.dump_progress = None;
    state.starting_slot = None;
    state.phase = Phase::Idle;
    state.status = format!("Failed to start: {err}");
    Task::none()
}

fn attach_process_session(
    state: &mut AppState,
    proc: crate::core::launch::ProcessHandle,
    tempdir: Option<TempDir>,
) -> Task<Message> {
    state.playing_handle = Some(PlayHandle::Process(proc.clone()));
    state.playing_tempdir = tempdir;
    state.phase = Phase::Playing;
    Task::perform(wait_process_task(proc), Message::GameExited)
}

fn attach_server_session(
    state: &mut AppState,
    join: tokio::task::JoinHandle<anyhow::Result<()>>,
    abort: tokio::task::AbortHandle,
    tempdir: Option<TempDir>,
) -> Task<Message> {
    state.playing_handle = Some(PlayHandle::Server { abort });
    state.playing_tempdir = tempdir;
    state.phase = Phase::Playing;
    Task::perform(wait_server_task(join), Message::GameExited)
}
