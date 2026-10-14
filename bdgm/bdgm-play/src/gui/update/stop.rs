use iced::Task;

use super::super::{
    message::Message,
    state::{AppState, Phase, PlayHandle},
    tasks::kill_process_task,
};

pub(super) fn handle_stop_game(state: &mut AppState) -> Task<Message> {
    // Modal closed by the user: kill whatever is running. The waiter
    // task then reports `GameExited`, which auto-closes the modal
    // and frees the port / temp dir.
    match state.playing_handle.clone() {
        Some(PlayHandle::Process(proc)) => handle_stop_process(state, proc),
        Some(PlayHandle::Server { abort }) => handle_stop_server(state, abort),
        None => handle_stop_idle(state),
    }
}

fn handle_stop_process(
    state: &mut AppState,
    proc: crate::core::launch::ProcessHandle,
) -> Task<Message> {
    state.status = String::from("Stopping game...");
    Task::perform(
        async move {
            kill_process_task(proc).await;
        },
        |_| Message::None,
    )
}

fn handle_stop_server(state: &mut AppState, abort: tokio::task::AbortHandle) -> Task<Message> {
    abort.abort();
    state.status = String::from("Stopping server...");
    Task::none()
}

fn handle_stop_idle(state: &mut AppState) -> Task<Message> {
    state.playing_tempdir = None;
    state.phase = Phase::Idle;
    state.status = String::from("");
    Task::none()
}
