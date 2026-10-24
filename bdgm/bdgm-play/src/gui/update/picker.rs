use iced::Task;
use rfd::AsyncFileDialog;

use super::super::{message::Message, state::AppState};

/// Shows the drive picker on Windows, or returns a mounted-disc folder picker task elsewhere.
pub(super) fn on_open_disc_clicked(state: &mut AppState) -> Task<Message> {
    #[cfg(not(windows))]
    let _ = state;

    if cfg!(windows) {
        #[cfg(windows)]
        {
            state.disc_picker_discs = Some(super::super::view::raw_disc::get_disc_drives());
            return Task::none();
        }
    }

    open_mounted_disc_picker()
}

/// Closes the drive picker without changing the current game status.
pub(super) fn on_disc_picker_cancel(state: &mut AppState) -> Task<Message> {
    state.disc_picker_discs = None;
    Task::none()
}

/// Returns an image file picker task without changing the current game status.
pub(super) fn on_open_image_clicked(_state: &mut AppState) -> Task<Message> {
    open_image_picker()
}

/// Returns a folder picker task that emits `RunMounted` on selection or `None` on cancellation.
fn open_mounted_disc_picker() -> Task<Message> {
    Task::perform(
        async {
            AsyncFileDialog::new()
                .set_title("Select mounted BDGM disc")
                .set_directory("~")
                .pick_folder()
                .await
                .map(|folder| folder.path().to_path_buf())
        },
        |path| match path {
            Some(path) => Message::RunMounted(path),
            None => Message::None,
        },
    )
}

/// Returns a filtered image picker task that emits `RunImage` on selection or `None` on cancellation.
fn open_image_picker() -> Task<Message> {
    Task::perform(
        async {
            AsyncFileDialog::new()
                .set_title("Select BDGM image")
                .set_directory("~")
                .add_filter("BDGM Image File", &["bin", "udf", "iso"])
                .pick_file()
                .await
                .map(|file| file.path().to_path_buf())
        },
        |path| match path {
            Some(path) => Message::RunImage(path),
            None => Message::None,
        },
    )
}
