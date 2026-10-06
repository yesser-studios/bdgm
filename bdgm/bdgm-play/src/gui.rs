use std::{path::PathBuf, process::ExitStatus};

use bdgm_play::{args::Args, image::resolve_image_args, run_sanitized};
use iced::{
    Element,
    Length::Fill,
    Task, Theme,
    widget::{button, container, row},
};
use rfd::{AsyncFileDialog, FileHandle};

#[derive(Debug, Default)]
struct AppState {}

#[derive(Debug, Clone)]
enum Message {
    #[cfg(windows)]
    OpenDiscFile,
    #[cfg(unix)]
    OpenDiscDirectory,
    OpenImageFile,
    #[cfg(windows)]
    OpenDisc(char),
    #[cfg(unix)]
    OpenDisc(PathBuf),
    OpenImage(PathBuf),
    None,
}

fn new() -> AppState {
    AppState {}
}

fn gen_dialog() -> AsyncFileDialog {
    AsyncFileDialog::new().set_directory("~")
}

fn extract_path(file: Option<FileHandle>) -> Option<PathBuf> {
    match file {
        Some(f) => Some(f.path().to_path_buf()),
        None => None,
    }
}

fn open_disc_folder() -> Task<Message> {
    return Task::perform(
        async || -> Option<PathBuf> {
            let res = gen_dialog()
                .set_title("Select mounted BDGM disc")
                .pick_folder()
                .await;
            extract_path(res)
        }(),
        |path| match path {
            Some(path) => Message::OpenDisc(path),
            None => Message::None,
        },
    );
}

fn open_image_file() -> Task<Message> {
    return Task::perform(
        async || -> Option<PathBuf> {
            let res = gen_dialog()
                .set_title("Select BDGM image")
                .add_filter("BDGM Image File", &["bin", "udf", "iso", "bdgm"])
                .pick_file()
                .await;
            extract_path(res)
        }(),
        |path| match path {
            Some(path) => Message::OpenImage(path),
            None => Message::None,
        },
    );
}

fn run_args(args: Args) -> Task<Message> {
    Task::perform(play(args), |r| {
        r.unwrap(); // TODO: replace unwrap with error handling
        Message::None
    })
}

fn update(_state: &mut AppState, message: Message) -> Task<Message> {
    let mut args = Args::new_imageless(None, None);
    match message {
        #[cfg(windows)]
        Message::OpenDisc(letter) => resolve_raw_disc(&mut args, letter),
        #[cfg(unix)]
        Message::OpenDisc(path) => {
            resolve_mounted_disc(&mut args, path);
            run_args(args)
        }
        Message::OpenImage(path) => {
            resolve_image(&mut args, path);
            run_args(args)
        }
        #[cfg(windows)]
        OpenRawDiscPicker => todo!(), // Does not use rfd
        #[cfg(unix)]
        Message::OpenDiscDirectory => open_disc_folder(),
        Message::OpenImageFile => open_image_file(),
        Message::None => Task::none(),
    }
}

fn view(_state: &AppState) -> Element<'_, Message> {
    container(
        row![
            button("Open Disc").on_press(Message::OpenDiscDirectory),
            button("Open Image").on_press(Message::OpenImageFile)
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
