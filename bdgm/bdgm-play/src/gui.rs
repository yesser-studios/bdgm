use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use bdgm::game::{Game, ValidatedGame};
use bdgm_play::{
    args::Args,
    dirs::GameDirs,
    image::resolve_image_args,
    install::install,
    launch::{kill_process, spawn_game_process, wait_for_process},
    read_manifest,
    server::start_html_server,
};
#[cfg(windows)]
use iced::widget::column;
use iced::{
    Element,
    Length::Fill,
    Task, Theme,
    font::{Font, Weight},
    widget::{button, container, row, text, text::Text},
};
use platform_dirs::AppDirs;
use rfd::{AsyncFileDialog, FileHandle};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// GAME INFO GROUNDWORK
//
// `GameInfo` is the display model for the running-game modal. It is built
// once at launch from `ValidatedGame` (+ the HTML server URL when relevant)
// and stored in `AppState::playing_info`. Extend this struct (e.g. cover art,
// description, playtime) to show more in the modal without touching the
// launch/kill plumbing.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
struct GameInfo {
    name: String,
    id: String,
    version: String,
    executable: String,
    runtime: String,
    runtime_version: Option<String>,
    /// Set for HTML games (server URL opened in the browser).
    url: Option<String>,
}

impl From<&ValidatedGame> for GameInfo {
    fn from(game: &ValidatedGame) -> Self {
        Self {
            name: game.name().to_owned(),
            id: game.id().to_owned(),
            version: game.version().to_owned(),
            executable: game.executable().to_string_lossy().into_owned(),
            runtime: game.runtime().to_string(),
            runtime_version: game.runtime_version().map(|s| s.to_owned()),
            url: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Phase {
    #[default]
    Idle,
    Starting,
    Playing,
}

// Killable handle for whatever is running behind the modal.
#[derive(Clone)]
enum PlayHandle {
    Process(bdgm_play::launch::ProcessHandle),
    Server { abort: tokio::task::AbortHandle },
}

impl std::fmt::Debug for PlayHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Process(_) => f.write_str("Process(..)"),
            Self::Server { .. } => f.write_str("Server(..)"),
        }
    }
}

// Session created by the background start task. Moved into `AppState` on
// `GameStarted(Ok)` via the slot below — never travels inside `Message`
// (which must stay `Clone`), so it can hold `JoinHandle` + `TempDir`.
struct StartedSession {
    handle: StartedHandle,
    tempdir: Option<TempDir>,
    info: GameInfo,
}

enum StartedHandle {
    Process(bdgm_play::launch::ProcessHandle),
    Server {
        join: tokio::task::JoinHandle<anyhow::Result<()>>,
        abort: tokio::task::AbortHandle,
    },
}

type Slot = Arc<Mutex<Option<StartedSession>>>;

struct AppState {
    #[cfg(windows)]
    show_drive_picker: bool,
    #[cfg(windows)]
    drives: Vec<char>,
    phase: Phase,
    status: String,
    starting_slot: Option<Slot>,
    playing_info: Option<GameInfo>,
    playing_handle: Option<PlayHandle>,
    playing_tempdir: Option<TempDir>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            #[cfg(windows)]
            show_drive_picker: false,
            #[cfg(windows)]
            drives: Vec::new(),
            phase: Phase::Idle,
            status: String::from(""),
            starting_slot: None,
            playing_info: None,
            playing_handle: None,
            playing_tempdir: None,
        }
    }
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("phase", &self.phase)
            .field("status", &self.status)
            .field("playing_info", &self.playing_info)
            .field("playing_handle", &self.playing_handle)
            .finish_non_exhaustive()
    }
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
    GameStarted(Result<GameInfo, String>),
    GameExited(Result<Option<i32>, String>),
    StopGame,
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

// Background start: resolve image -> manifest -> validate -> install ->
// spawn killable process or HTML server. Stores the session in `slot` and
// returns only the display info so `Message` stays `Clone`.
async fn start_game_task(args: Args, slot: Slot) -> Result<GameInfo, String> {
    let result: anyhow::Result<GameInfo> = (|| async {
        let (resolved, tempdir) = resolve_image_args(args)?;
        let manifest = read_manifest(&resolved)?;
        let game = Game::from_str(&manifest)?;
        let game = ValidatedGame::validate(game)?;

        let app_dirs =
            AppDirs::new(Some("bdgm-play"), true).ok_or_else(|| anyhow::anyhow!("NoAppDirs"))?;
        // NOTE: runs on the async runtime; large installs will briefly block
        // the UI. Future work: move into `spawn_blocking` with progress.
        install(&game, &app_dirs, &resolved)?;

        let mut info = GameInfo::from(&game);
        let handle = match game.runtime() {
            bdgm::runtime::Runtime::HTML => {
                let game_dirs = GameDirs::from(&game, &app_dirs);
                let server = start_html_server(&game, &game_dirs.install, &app_dirs, false).await?;
                info.url = Some(server.url.clone());
                StartedHandle::Server {
                    join: server.join,
                    abort: server.abort,
                }
            }
            _ => {
                let proc = spawn_game_process(&game, &resolved, &app_dirs)?;
                StartedHandle::Process(proc)
            }
        };

        slot.lock()
            .map_err(|_| anyhow::anyhow!("starting slot poisoned"))?
            .replace(StartedSession {
                handle,
                tempdir,
                info: info.clone(),
            });

        Ok(info)
    })()
    .await;

    result.map_err(|e| format!("{e:#}"))
}

async fn wait_process_task(
    handle: bdgm_play::launch::ProcessHandle,
) -> Result<Option<i32>, String> {
    wait_for_process(handle).await.map_err(|e| format!("{e:#}"))
}

async fn wait_server_task(
    join: tokio::task::JoinHandle<anyhow::Result<()>>,
) -> Result<Option<i32>, String> {
    match join.await {
        Ok(Ok(())) => Ok(None),
        Ok(Err(e)) => Err(format!("Server crashed: {e:#}")),
        Err(join_err) if join_err.is_cancelled() => Ok(None),
        Err(join_err) => Err(format!("Server task failed: {join_err}")),
    }
}

async fn kill_process_task(handle: bdgm_play::launch::ProcessHandle) {
    kill_process(&handle).await;
}

fn is_busy(state: &AppState) -> bool {
    matches!(state.phase, Phase::Starting | Phase::Playing)
}

fn begin_launch(state: &mut AppState, args: Args) -> Task<Message> {
    if is_busy(state) {
        state.status = String::from("A game is already running — stop it first.");
        return Task::none();
    }
    let slot: Slot = Arc::new(Mutex::new(None));
    state.starting_slot = Some(Arc::clone(&slot));
    state.phase = Phase::Starting;
    state.status = String::from("Starting game...");
    Task::perform(start_game_task(args, slot), Message::GameStarted)
}

fn take_starting_session(state: &mut AppState) -> Option<StartedSession> {
    state.starting_slot.take()?.lock().ok()?.take()
}

#[allow(unused_variables)]
fn update(state: &mut AppState, message: Message) -> Task<Message> {
    let mut args = Args::new_imageless(None, None);
    match message {
        #[cfg(windows)]
        Message::OpenDrivePicker => {
            if is_busy(state) {
                state.status = String::from("A game is already running — stop it first.");
                return Task::none();
            }
            state.drives = list_candidate_drives();
            state.show_drive_picker = true;
            Task::none()
        }
        #[cfg(windows)]
        Message::OpenDiscRaw(letter) => {
            state.show_drive_picker = false;
            resolve_raw_disc(&mut args, letter);
            begin_launch(state, args)
        }
        #[cfg(windows)]
        Message::DrivePickerClosed => {
            state.show_drive_picker = false;
            Task::none()
        }
        #[cfg(unix)]
        Message::OpenDiscMounted(path) => {
            resolve_mounted_disc(&mut args, path);
            begin_launch(state, args)
        }
        Message::OpenImage(path) => {
            resolve_image(&mut args, path);
            begin_launch(state, args)
        }
        #[cfg(unix)]
        Message::OpenDiscDirectory => {
            if is_busy(state) {
                state.status = String::from("A game is already running — stop it first.");
                return Task::none();
            }
            open_disc_folder()
        }
        Message::OpenImageFile => {
            if is_busy(state) {
                state.status = String::from("A game is already running — stop it first.");
                return Task::none();
            }
            open_image_file()
        }
        Message::GameStarted(result) => match result {
            Ok(_) => {
                let Some(session) = take_starting_session(state) else {
                    state.phase = Phase::Idle;
                    state.status = String::from("Game started but session was lost.");
                    return Task::none();
                };
                state.status = format!("Running: {}", session.info.name);
                state.playing_info = Some(session.info.clone());
                match session.handle {
                    StartedHandle::Process(proc) => {
                        state.playing_handle = Some(PlayHandle::Process(proc.clone()));
                        state.playing_tempdir = session.tempdir;
                        state.phase = Phase::Playing;
                        Task::perform(wait_process_task(proc), Message::GameExited)
                    }
                    StartedHandle::Server { join, abort } => {
                        state.playing_handle = Some(PlayHandle::Server { abort });
                        state.playing_tempdir = session.tempdir;
                        state.phase = Phase::Playing;
                        Task::perform(wait_server_task(join), Message::GameExited)
                    }
                }
            }
            Err(err) => {
                state.starting_slot = None;
                state.phase = Phase::Idle;
                state.status = format!("Failed to start: {err}");
                Task::none()
            }
        },
        Message::GameExited(result) => {
            // Natural exit or server crash: auto-close the modal by leaving
            // `Playing`. Dropping `playing_tempdir` cleans the extract dir;
            // the port was already freed because the server task ended.
            state.playing_handle = None;
            state.playing_tempdir = None;
            state.starting_slot = None;
            state.phase = Phase::Idle;
            match result {
                Ok(code) => {
                    let name = state
                        .playing_info
                        .as_ref()
                        .map(|i| i.name.clone())
                        .unwrap_or_else(|| String::from("Game"));
                    state.status = match code {
                        Some(0) | None => format!("{name} exited."),
                        Some(c) => format!("{name} exited with code {c}."),
                    };
                }
                Err(err) => {
                    state.status = format!("Game ended: {err}");
                }
            }
            Task::none()
        }
        Message::StopGame => {
            // Modal closed by the user: kill whatever is running. The waiter
            // task then reports `GameExited`, which auto-closes the modal
            // and frees the port / temp dir.
            match state.playing_handle.clone() {
                Some(PlayHandle::Process(proc)) => {
                    state.status = String::from("Stopping game...");
                    Task::perform(
                        async move {
                            kill_process_task(proc).await;
                        },
                        |_| Message::None,
                    )
                }
                Some(PlayHandle::Server { abort }) => {
                    abort.abort();
                    state.status = String::from("Stopping server...");
                    Task::none()
                }
                None => {
                    state.playing_tempdir = None;
                    state.phase = Phase::Idle;
                    state.status = String::from("");
                    Task::none()
                }
            }
        }
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

// GAME INFO GROUNDWORK: extend this card (art, description, version history)
// — all data comes from `AppState::playing_info: Option<GameInfo>`.
fn game_modal<'a>(info: &'a GameInfo) -> Element<'a, Message> {
    let mut card = iced::widget::column![
        text(format!("Running: {} version {}", info.name, info.version)).size(22),
        text(format!("ID: {}", info.id)).size(14),
    ]
    .spacing(4)
    .padding(16);

    if let Some(url) = &info.url {
        card = card.push(text(format!("Serving at {url}")).size(14));
    }

    card = card.push(
        row![
            button(button_text("Stop")).on_press(Message::StopGame),
            button(button_text("Close")).on_press(Message::StopGame),
        ]
        .spacing(10),
    );

    container(card).padding(8).into()
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

    let busy = is_busy(state);

    #[cfg(windows)]
    let open_disc_button = button(button_text("Open Disc")).on_press_maybe(if busy {
        None
    } else {
        Some(Message::OpenDrivePicker)
    });
    #[cfg(unix)]
    let open_disc_button = button(centered_label("Open Disc"))
        .width(150)
        .height(150)
        .on_press_maybe(if busy {
            None
        } else {
            Some(Message::OpenDiscDirectory)
        });

    let open_image_button = button(centered_label("Open Image"))
        .width(150)
        .height(150)
        .on_press_maybe(if busy {
            None
        } else {
            Some(Message::OpenImageFile)
        });

    let mut main = iced::widget::column![row![open_disc_button, open_image_button].spacing(10),]
        .spacing(10)
        .padding(10);

    match state.phase {
        Phase::Idle if state.status.is_empty() => {}
        Phase::Idle => main = main.push(text(state.status.clone()).size(14)),
        Phase::Starting => main = main.push(text("Starting game...").size(14)),
        Phase::Playing => {
            if let Some(info) = &state.playing_info {
                main = main.push(game_modal(info));
            }
        }
    }

    container(main).center_x(Fill).center_y(Fill).into()
}

pub fn run_gui(_args: Args) -> iced::Result {
    iced::application(new, update, view)
        .theme(|_: &AppState| Theme::CatppuccinMocha)
        .run()
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
