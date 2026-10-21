use iced::{
    Alignment, Element, Length,
    widget::{ProgressBar, container, row, text},
};

use super::{
    message::Message,
    session::is_busy,
    state::{AppState, Phase},
};

mod buttons;
mod components;
mod modal;

use buttons::{open_disc_button, open_image_button};
use modal::game_modal;

#[cfg(windows)]
use components::button_text;

pub(crate) fn view(state: &AppState) -> Element<'_, Message> {
    #[cfg(windows)]
    if state.show_drive_picker {
        return drive_picker_view(state);
    }

    let busy = is_busy(state);

    let open_disc_button = open_disc_button(busy);
    let open_image_button = open_image_button(busy);

    let mut main = iced::widget::column![row![open_disc_button, open_image_button].spacing(10),]
        .spacing(10)
        .padding(10)
        .align_x(Alignment::Center);

    match state.phase {
        Phase::Idle if state.status.is_empty() => {}
        Phase::Idle => main = main.push(text(state.status.clone()).size(14)),
        Phase::Starting => {
            main = main.push(text(state.status.clone()).size(14));
            if let Some((done, total)) = state.dump_progress {
                let fraction = if total > 0 {
                    (done as f32 / total as f32).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                main = main.push(ProgressBar::new(0.0..=1.0, fraction).length(350));
            }
        }
        Phase::Playing => {
            if let Some(info) = &state.playing_info {
                main = main.push(game_modal(info));
            }
        }
    }

    container(main)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

#[cfg(windows)]
fn drive_picker_view(state: &AppState) -> Element<'_, Message> {
    let mut col = iced::widget::column![text("Select disc drive:")]
        .spacing(10)
        .align_x(Alignment::Center);
    for drive in &state.drives {
        col = col.push(
            iced::widget::button(button_text(format!("Drive {drive}:")))
                .on_press(Message::OpenDiscRaw(*drive)),
        );
    }
    col =
        col.push(iced::widget::button(button_text("Cancel")).on_press(Message::DrivePickerClosed));
    container(col)
        .padding(10)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}
