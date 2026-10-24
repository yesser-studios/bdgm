use iced::Task;

use super::{message::Message, state::AppState};

mod launch;
mod picker;
mod running;
mod state;

pub(super) fn update(state: &mut AppState, message: Message) -> Task<Message> {
    match message {
        Message::OpenDiscClicked => picker::on_open_disc_clicked(state),
        Message::OpenImageClicked => picker::on_open_image_clicked(state),
        Message::DiscPickerCancel => picker::on_disc_picker_cancel(state),
        Message::RunRawDisc(disc) => {
            let request = launch::create_raw_disc_launch_request(state, disc);
            launch::pre_launch(state);
            launch::launch(request)
        }
        Message::RunMounted(path) => {
            let request = launch::create_mounted_launch_request(state, path);
            launch::pre_launch(state);
            launch::launch(request)
        }
        Message::RunImage(path) => {
            let request = launch::create_image_launch_request(state, path);
            launch::pre_launch(state);
            launch::launch(request)
        }
        Message::StateChanged(new_state) => state::on_state_changed(state, new_state),
        Message::GameStarted(game, handle) => running::on_game_started(state, game, handle),
        Message::StopGameClicked => running::on_stop_game_clicked(state),
        Message::OpenHTMLGameClicked => running::on_open_html_game_clicked(state),
        Message::GameExited(game, exit_code) => running::on_game_exited(state, game, exit_code),
        Message::GameError(error) => running::on_error(state, error),
        Message::None => Task::none(),
    }
}
