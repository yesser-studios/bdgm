use std::{path::PathBuf, process::ExitStatus};

use bdgm_play::{args::Args, image::resolve_image_args, run_sanitized};
#[cfg(windows)]
use iced::widget::column;
use iced::{
    Element,
    Length::Fill,
    Task, Theme,
    font::{Font, Weight},
    widget::{button, container, row, text, text::Text},
};
use rfd::{AsyncFileDialog, FileHandle};

#[derive(Debug, Default)]
struct AppState {
    #[cfg(windows)]
    show_drive_picker: bool,
    #[cfg(windows)]
    drives: Vec<char>,
}

#[derive(Debug, Clone)]
enum Message {
    #[cfg(windows)]
    OpenDrivePicker,
    #[cfg(unix)]
    OpenDiscDirectory,
    OpenImageFile,
    #[cfg(windows)]
    OpenDiscRaw(char),
    #[cfg(unix)]
    OpenDiscMounted(PathBuf),
    OpenImage(PathBuf),
    #[cfg(windows)]
    DrivePickerClosed,
    None,
}

fn new() -> AppState {
    AppState::default()
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

#[cfg(windows)]
fn list_candidate_drives() -> Vec<char> {
    use windows_sys::Win32::Storage::FileSystem::{DRIVE_CDROM, GetDriveTypeW, GetLogicalDrives};

    // SAFETY: GetLogicalDrives takes no args and returns a bitmask.
    let mask = unsafe { GetLogicalDrives() };
    if mask == 0 {
        return Vec::new();
    }

    let mut all = Vec::new();
    let mut cdrom = Vec::new();
    for i in 0..26u32 {
        if (mask & (1 << i)) == 0 {
            continue;
        }
        let letter = (b'A' + i as u8) as char;
        // NUL-terminated "X:\" UTF-16 for GetDriveTypeW.
        let path: [u16; 4] = [letter as u16, b':' as u16, b'\\' as u16, 0];
        // SAFETY: path is a valid NUL-terminated UTF-16 root path.
        let drive_type = unsafe { GetDriveTypeW(path.as_ptr()) };
        all.push(letter);
        if drive_type == DRIVE_CDROM {
            cdrom.push(letter);
        }
    }
    if cdrom.is_empty() { all } else { cdrom }
}

#[cfg(unix)]
fn open_disc_folder() -> Task<Message> {
    Task::perform(
        async {
            let res = gen_dialog()
                .set_title("Select mounted BDGM disc")
                .pick_folder()
                .await;
            extract_path(res)
        },
        |path| match path {
            Some(path) => Message::OpenDiscMounted(path),
            None => Message::None,
        },
    )
}

fn open_image_file() -> Task<Message> {
    Task::perform(
        async {
            let res = gen_dialog()
                .set_title("Select BDGM image")
                .add_filter("BDGM Image File", &["bin", "udf", "iso"])
                .pick_file()
                .await;
            extract_path(res)
        },
        |path| match path {
            Some(path) => Message::OpenImage(path),
            None => Message::None,
        },
    )
}

fn run_args(args: Args) -> Task<Message> {
    Task::perform(play(args), |r| {
        r.unwrap(); // TODO: replace unwrap with error handling
        Message::None
    })
}

#[allow(unused_variables)]
fn update(state: &mut AppState, message: Message) -> Task<Message> {
    let mut args = Args::new_imageless(None, None);
    match message {
        #[cfg(windows)]
        Message::OpenDrivePicker => {
            state.drives = list_candidate_drives();
            state.show_drive_picker = true;
            Task::none()
        }
        #[cfg(windows)]
        Message::OpenDiscRaw(letter) => {
            state.show_drive_picker = false;
            resolve_raw_disc(&mut args, letter);
            run_args(args)
        }
        #[cfg(windows)]
        Message::DrivePickerClosed => {
            state.show_drive_picker = false;
            Task::none()
        }
        #[cfg(unix)]
        Message::OpenDiscMounted(path) => {
            resolve_mounted_disc(&mut args, path);
            run_args(args)
        }
        Message::OpenImage(path) => {
            resolve_image(&mut args, path);
            run_args(args)
        }
        #[cfg(unix)]
        Message::OpenDiscDirectory => open_disc_folder(),
        Message::OpenImageFile => open_image_file(),
        Message::None => Task::none(),
    }
}

const BUTTON_TEXT_SIZE: f32 = 20.0;

fn button_text<'a>(label: impl Into<String>) -> Text<'a> {
    text(label.into()).size(BUTTON_TEXT_SIZE).font(Font {
        weight: Weight::Bold,
        ..Font::DEFAULT
    })
}

fn centered_label<'a>(label: impl Into<String>) -> Element<'a, Message> {
    container(button_text(label))
        .center_x(Fill)
        .center_y(Fill)
        .into()
}

#[allow(unused_variables)]
fn view(state: &AppState) -> Element<'_, Message> {
    #[cfg(windows)]
    if state.show_drive_picker {
        let mut col = column![text("Select disc drive:")].spacing(10);
        for drive in &state.drives {
            col = col.push(
                button(button_text(format!("Drive {drive}:")))
                    .on_press(Message::OpenDiscRaw(*drive)),
            );
        }
        col = col.push(button(button_text("Cancel")).on_press(Message::DrivePickerClosed));
        return container(col)
            .padding(10)
            .center_x(Fill)
            .center_y(Fill)
            .into();
    }

    #[cfg(windows)]
    let open_disc_button = button(button_text("Open Disc")).on_press(Message::OpenDrivePicker);
    #[cfg(unix)]
    let open_disc_button = button(centered_label("Open Disc"))
        .width(150)
        .height(150)
        .on_press(Message::OpenDiscDirectory);

    container(
        row![
            open_disc_button,
            button(centered_label("Open Image"))
                .width(150)
                .height(150)
                .on_press(Message::OpenImageFile)
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
        .theme(|_: &AppState| Theme::CatppuccinMocha)
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

    args.location = Some(path);
    args.set_raw_disc(true);
    args.image = false;
}

pub(crate) fn resolve_image(args: &mut Args, path: PathBuf) {
    args.location = Some(path);
    args.image = true;
    args.set_raw_disc(false);
}
