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
            let r = Some(Message::OpenDiscDirectory);
            #[cfg(windows)]
            let r = Some(Message::OpenDrivePicker);
            r
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
