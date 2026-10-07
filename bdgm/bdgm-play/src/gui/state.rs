use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use bdgm::game::ValidatedGame;
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
// Unread fields are display-model groundwork for extending the modal.
#[allow(dead_code)]
pub(crate) struct GameInfo {
    pub(crate) name: String,
    pub(crate) id: String,
    pub(crate) version: String,
    pub(crate) executable: String,
    pub(crate) runtime: String,
    pub(crate) runtime_version: Option<String>,
    /// Set for HTML games (server URL opened in the browser).
    pub(crate) url: Option<String>,
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
pub(crate) enum Phase {
    #[default]
    Idle,
    Starting,
    Playing,
}

// Killable handle for whatever is running behind the modal.
#[derive(Clone)]
pub(crate) enum PlayHandle {
    Process(crate::core::launch::ProcessHandle),
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
pub(crate) struct StartedSession {
    pub(crate) handle: StartedHandle,
    pub(crate) tempdir: Option<TempDir>,
    pub(crate) info: GameInfo,
}

pub(crate) enum StartedHandle {
    Process(crate::core::launch::ProcessHandle),
    Server {
        join: tokio::task::JoinHandle<anyhow::Result<()>>,
        abort: tokio::task::AbortHandle,
    },
}

pub(crate) type Slot = Arc<Mutex<Option<StartedSession>>>;

pub(crate) struct AppState {
    #[cfg(windows)]
    pub(crate) show_drive_picker: bool,
    #[cfg(windows)]
    pub(crate) drives: Vec<char>,
    /// Runtime override from CLI `--runtime`, preserved across game selection.
    pub(crate) initial_runtime: Option<PathBuf>,
    pub(crate) phase: Phase,
    pub(crate) status: String,
    /// Raw-disc dump progress as `(sectors_done, sectors_total)`.
    /// `Some` while dumping on Windows; `None` otherwise.
    pub(crate) dump_progress: Option<(u64, u64)>,
    pub(crate) starting_slot: Option<Slot>,
    pub(crate) playing_info: Option<GameInfo>,
    pub(crate) playing_handle: Option<PlayHandle>,
    pub(crate) playing_tempdir: Option<TempDir>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            #[cfg(windows)]
            show_drive_picker: false,
            #[cfg(windows)]
            drives: Vec::new(),
            initial_runtime: None,
            phase: Phase::Idle,
            status: String::from(""),
            dump_progress: None,
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
