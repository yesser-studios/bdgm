use std::path::PathBuf;

use super::state::GameInfo;

#[derive(Debug, Clone)]
pub(crate) enum Message {
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
