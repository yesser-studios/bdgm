use iced::widget::{Button, button};

use crate::gui::message::Message;

#[cfg(windows)]
use super::components::button_text;
use super::components::centered_label;

#[cfg(windows)]
pub(super) fn open_disc_button(busy: bool) -> Button<'static, Message> {
    button(button_text("Open Disc")).on_press_maybe(if busy {
        None
    } else {
        Some(Message::OpenDrivePicker)
    })
}

#[cfg(unix)]
pub(super) fn open_disc_button(busy: bool) -> Button<'static, Message> {
    button(centered_label("Open Disc"))
        .width(150)
        .height(150)
        .on_press_maybe(if busy {
            None
        } else {
            Some(Message::OpenDiscDirectory)
        })
}

pub(super) fn open_image_button(busy: bool) -> Button<'static, Message> {
    button(centered_label("Open Image"))
        .width(150)
        .height(150)
        .on_press_maybe(if busy {
            None
        } else {
            Some(Message::OpenImageFile)
        })
}
