/// Gets all CDROM disc drives. If there are none, gets all attached drives.
pub(in crate::gui) fn get_disc_drives() -> Vec<char> {
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

    for index in 0..26u32 {
        if (mask & (1 << index)) == 0 {
            continue;
        }

        let letter = (b'A' + index as u8) as char;
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
