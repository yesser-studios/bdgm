use iced::{
    Element, Font,
    Length::Fill,
    font::Weight,
    widget::{container, text},
};

use crate::gui::message::Message;

const BUTTON_TEXT_SIZE: f32 = 20.0;

pub(super) fn centered_button_text<'a>(label: impl Into<String>) -> Element<'a, Message> {
    container(text(label.into()).size(BUTTON_TEXT_SIZE).font(Font {
        weight: Weight::Bold,
        ..Font::DEFAULT
    }))
    .center_x(Fill)
    .center_y(Fill)
    .into()
}
