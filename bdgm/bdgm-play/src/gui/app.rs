use iced::{
    Theme,
    window::{self, icon},
};

use crate::cli::args::Args;

use super::{state::AppState, update::update, view::view};

fn new() -> AppState {
    AppState::default()
}

pub fn run_gui(_args: Args) -> iced::Result {
    iced::application(new, update, view)
        .theme(|_: &AppState| Theme::CatppuccinMocha)
        .title("BDGM Player")
        .window(window::Settings {
            icon: icon::from_file_data(include_bytes!("../../../../logo.png"), None).ok(),
            size: iced::Size {
                width: 700.0,
                height: 500.0,
            },
            min_size: Some(iced::Size {
                width: 400.0,
                height: 300.0,
            }),
            ..Default::default()
        })
        .run()
}
