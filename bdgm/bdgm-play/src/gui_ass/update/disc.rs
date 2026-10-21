#[cfg(unix)]
use std::path::PathBuf;

use iced::Task;

use crate::cli::args::Args;

#[cfg(unix)]
use super::super::{args_resolve::resolve_mounted_disc, dialogs::open_disc_folder};
#[cfg(windows)]
use super::super::{args_resolve::resolve_raw_disc, dialogs::list_candidate_drives};
use super::super::{
    message::Message,
    session::{begin_launch, ensure_not_busy},
    state::AppState,
};

#[cfg(windows)]
pub(super) fn handle_open_drive_picker(state: &mut AppState) -> Task<Message> {
    if ensure_not_busy(state) {
        return Task::none();
    }
    state.drives = list_candidate_drives();
    state.show_drive_picker = true;
    Task::none()
}

#[cfg(windows)]
pub(super) fn handle_open_disc_raw(state: &mut AppState, letter: char) -> Task<Message> {
    state.show_drive_picker = false;
    let mut args = Args::new_imageless(None, state.initial_runtime.clone());
    resolve_raw_disc(&mut args, letter);
    begin_launch(state, args)
}

#[cfg(windows)]
pub(super) fn handle_drive_picker_closed(state: &mut AppState) -> Task<Message> {
    state.show_drive_picker = false;
    Task::none()
}

#[cfg(unix)]
pub(super) fn handle_open_disc_mounted(state: &mut AppState, path: PathBuf) -> Task<Message> {
    let mut args = Args::new_imageless(None, state.initial_runtime.clone());
    resolve_mounted_disc(&mut args, path);
    begin_launch(state, args)
}

#[cfg(unix)]
pub(super) fn handle_open_disc_directory(state: &mut AppState) -> Task<Message> {
    if ensure_not_busy(state) {
        return Task::none();
    }
    open_disc_folder()
}
