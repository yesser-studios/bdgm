use std::path::PathBuf;

use iced::Task;
use rfd::{AsyncFileDialog, FileHandle};

use super::message::Message;

pub(crate) fn gen_dialog() -> AsyncFileDialog {
    AsyncFileDialog::new().set_directory("~")
}

pub(crate) fn extract_path(file: Option<FileHandle>) -> Option<PathBuf> {
    file.map(|f| f.path().to_path_buf())
}

#[cfg(windows)]
pub(crate) fn list_candidate_drives() -> Vec<char> {
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};

    // windows-sys does not export DRIVE_* constants, so define locally.
    // Values from Win32 fileapi.h: 0 UNKNOWN, 1 NO_ROOT, 2 REMOVABLE,
    // 3 FIXED, 4 REMOTE, 5 CDROM, 6 RAMDISK.
    const DRIVE_CDROM: u32 = 5;

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
pub(crate) fn open_disc_folder() -> Task<Message> {
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

pub(crate) fn open_image_file() -> Task<Message> {
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
