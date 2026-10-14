use iced::{
    Alignment, Element,
    Length::Fill,
    widget::{button, container, text},
};

use crate::gui::{message::Message, state::GameInfo};

use super::components::button_text;

// GAME INFO GROUNDWORK: extend this card (art, description, version history)
// — all data comes from `AppState::playing_info: Option<GameInfo>`.
pub(super) fn game_modal<'a>(info: &'a GameInfo) -> Element<'a, Message> {
    let mut card = iced::widget::column![
        text(format!("Running: {} version {}", info.name, info.version)).size(22),
        text(format!("ID: {}", info.id)).size(14),
    ]
    .spacing(4)
    .padding(16)
    .align_x(Alignment::Center);

    if let Some(url) = &info.url {
        card = card.push(text(format!("Serving at {url}")).size(14));
    }

    card = card.push(button(button_text("Stop")).on_press(Message::StopGame));

    container(card).padding(8).width(Fill).center_x(Fill).into()
}
