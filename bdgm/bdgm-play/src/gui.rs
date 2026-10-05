use std::{path::PathBuf, process::ExitStatus};

use bdgm_play::{args::Args, image::resolve_image_args, run_sanitized};
use iced::{
    Element,
    Length::Fill,
    Task, Theme,
    widget::{button, container, row},
};

#[derive(Debug, Default)]
struct AppState {}

#[derive(Debug, Clone)]
enum Message {
    #[cfg(windows)]
    OpenDisc(char),
    #[cfg(unix)]
    OpenDisc(PathBuf),
    OpenImage(PathBuf),
    GamePlayed,
}

fn new() -> AppState {
    AppState {}
}

fn update(_state: &mut AppState, message: Message) -> Task<Message> {
    let mut args = Args::new_imageless(None, None);
    match message {
        #[cfg(windows)]
        Message::OpenDisc(letter) => resolve_raw_disc(&mut args, letter),
        #[cfg(unix)]
        Message::OpenDisc(path) => resolve_mounted_disc(&mut args, path),
        Message::OpenImage(path) => resolve_image(&mut args, path),
        Message::GamePlayed => return Task::none(),
    }

    Task::perform(play(args), |r| {
        r.unwrap(); // TODO: replace unwrap with error handling
        Message::GamePlayed
    })
}

fn view(_state: &AppState) -> Element<'_, Message> {
    container(
        row![
            button("Open Disc").on_press(Message::OpenDisc(todo!())),
            button("Open Image").on_press(Message::OpenImage(todo!()))
        ]
        .spacing(10),
    )
    .padding(10)
    .center_x(Fill)
    .center_y(Fill)
    .into()
}

pub fn run_gui(_args: Args) -> iced::Result {
    iced::application(new, update, view)
        .theme(|_: &AppState| Theme::Dark)
        .run()
}

async fn play(args: Args) -> anyhow::Result<ExitStatus> {
    let (args, tempdir) = resolve_image_args(args)?;
    let result = run_sanitized(args, false).await;
    drop(tempdir);

    result
}

#[allow(unused)]
pub(crate) fn resolve_mounted_disc(args: &mut Args, path: PathBuf) {
    args.location = Some(path);
    args.image = false;
    args.set_raw_disc(false);
}

#[allow(unused)]
pub(crate) fn resolve_raw_disc(args: &mut Args, drive_letter: char) {
    let path = PathBuf::from(format!("\\\\.\\{drive_letter}:"));
    dbg!(&path);

    args.location = Some(path);
    args.set_raw_disc(true);
    args.image = false;
}

pub(crate) fn resolve_image(args: &mut Args, path: PathBuf) {
    args.location = Some(path);
    args.image = true;
    args.set_raw_disc(false);
}
