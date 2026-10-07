use iced::widget::{Button, button};

use crate::gui::message::Message;

use super::components::centered_label;

pub(super) fn open_disc_button(busy: bool) -> Button<'static, Message> {
    button(centered_label("Open Disc"))
        .width(150)
        .height(50)
        .on_press_maybe(if busy {
            None
        } else {
            #[cfg(unix)]
            return Some(Message::OpenDiscDirectory);
            #[cfg(windows)]
            Some(Message::OpenDrivePicker)
        })
}

pub(super) fn open_image_button(busy: bool) -> Button<'static, Message> {
    button(centered_label("Open Image"))
        .width(150)
        .height(50)
        .on_press_maybe(if busy {
            None
        } else {
            Some(Message::OpenImageFile)
        })
}
