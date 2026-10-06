use iced::Theme;

use crate::cli::args::Args;

use super::{state::AppState, update::update, view::view};

fn new() -> AppState {
    AppState::default()
}

pub fn run_gui(_args: Args) -> iced::Result {
    iced::application(new, update, view)
        .theme(|_: &AppState| Theme::CatppuccinMocha)
        .run()
}
