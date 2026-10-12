use iced::Task;

use crate::gui::{message::Message, state::AppState};

mod disc;
mod exited;
mod image;
mod progress;
mod started;
mod stop;

pub(crate) fn update(state: &mut AppState, message: Message) -> Task<Message> {
    match message {
        #[cfg(windows)]
        Message::OpenDrivePicker => disc::handle_open_drive_picker(state),
        #[cfg(windows)]
        Message::OpenDiscRaw(letter) => disc::handle_open_disc_raw(state, letter),
        #[cfg(windows)]
        Message::DrivePickerClosed => disc::handle_drive_picker_closed(state),
        #[cfg(unix)]
        Message::OpenDiscMounted(path) => disc::handle_open_disc_mounted(state, path),
        Message::OpenImage(path) => image::handle_open_image(state, path),
        Message::DumpProgress(done, total) => progress::handle_dump_progress(state, done, total),
        #[cfg(unix)]
        Message::OpenDiscDirectory => disc::handle_open_disc_directory(state),
        Message::OpenImageFile => image::handle_open_image_file(state),
        Message::GameStarted(result) => started::handle_game_started(state, result),
        Message::GameExited(result) => exited::handle_game_exited(state, result),
        Message::StopGame => stop::handle_stop_game(state),
        Message::None => Task::none(),
    }
}
