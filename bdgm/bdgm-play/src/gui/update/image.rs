use std::path::PathBuf;

use iced::Task;

use crate::cli::args::Args;

use super::super::{
    args_resolve::resolve_image,
    dialogs::open_image_file,
    message::Message,
    session::{begin_launch, ensure_not_busy},
    state::AppState,
};

pub(super) fn handle_open_image(state: &mut AppState, path: PathBuf) -> Task<Message> {
    let mut args = Args::new_imageless(None, state.initial_runtime.clone());
    resolve_image(&mut args, path);
    begin_launch(state, args)
}

pub(super) fn handle_open_image_file(state: &mut AppState) -> Task<Message> {
    if ensure_not_busy(state) {
        return Task::none();
    }
    open_image_file()
}
