use std::sync::Arc;

use iced::Task;
use tokio::sync::mpsc::UnboundedSender;

use crate::core::{AppError, State, StateHandler};

use super::super::{
    message::Message,
    state::{AppState, Status},
};

pub(super) struct GuiStateHandler {
    current_state: Option<Arc<State>>,
    last_state: Option<Arc<State>>,
    sender: UnboundedSender<Message>,
}

impl GuiStateHandler {
    pub(super) fn new(sender: UnboundedSender<Message>) -> Self {
        Self {
            current_state: None,
            last_state: None,
            sender,
        }
    }
}

impl StateHandler for GuiStateHandler {
    fn change_state(&mut self, state: State) {
        let state = Arc::new(state);
        self.last_state = self.current_state.replace(Arc::clone(&state));
        let _ = self.sender.send(Message::StateChanged(state));
    }

    fn revert_state(&mut self) {
        if self.last_state.is_none() {
            return;
        }

        std::mem::swap(&mut self.current_state, &mut self.last_state);

        if let Some(state) = &self.current_state {
            let _ = self.sender.send(Message::StateChanged(Arc::clone(state)));
        }
    }
}

pub(super) fn on_state_changed(state: &mut AppState, new_state: Arc<State>) -> Task<Message> {
    if !matches!(state.status, Status::Starting(_)) {
        return Task::none();
    }

    let starting = Arc::clone(&new_state);
    state.status = match new_state.as_ref() {
        State::Error(error) => Status::Error(Arc::new(AppError::Other(error.to_string()))),
        _ => Status::Starting(starting),
    };

    Task::none()
}
