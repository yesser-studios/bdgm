use iced::{
    Element,
    Length::Fill,
    font::{Font, Weight},
    widget::{container, text, text::Text},
};

use crate::gui::message::Message;

pub(super) const BUTTON_TEXT_SIZE: f32 = 20.0;

pub(super) fn button_text<'a>(label: impl Into<String>) -> Text<'a> {
    text(label.into()).size(BUTTON_TEXT_SIZE).font(Font {
        weight: Weight::Bold,
        ..Font::DEFAULT
    })
}

pub(super) fn centered_label<'a>(label: impl Into<String>) -> Element<'a, Message> {
    container(button_text(label))
        .center_x(Fill)
        .center_y(Fill)
        .into()
}
