//! Windows console handling for GUI launches.
//!
//! The `bdgm-play` binary keeps the `console` subsystem so CLI usage
//! (`bdgm-play <path>`, `--help`, errors) always has somewhere to print.
//! That means Windows also creates a console window for GUI launches from
//! Explorer / shortcuts / the Start menu, which is undesirable.
//!
//! [`hide_console_if_launched_without_shell`] hides that window, but only
//! when the process owns the console alone (i.e. it was *not* started from an
//! existing shell such as `cmd`, PowerShell, or Windows Terminal, where the
//! console is shared and must be kept for log output).
//!
//! The console is hidden with `ShowWindow(SW_HIDE)` rather than detached with
//! `FreeConsole`: detaching destroys the console while our stdin/stdout/stderr
//! handles still point at it, so later child-game spawns with inherited stdio
//! fail with OS error 50 (`ERROR_NOT_SUPPORTED`). Hiding keeps the console
//! attached (handles stay valid, games inherit the hidden console instead of
//! popping their own visible one) while making it invisible.

/// Hide the console window if the GUI was launched outside of a shell.
///
/// Returns `true` if the console was hidden, `false` otherwise
/// (non-Windows, no console attached, shared console, or API failure --
/// in all of these cases keeping the console visible is the safe fallback).
#[cfg(windows)]
pub fn hide_console_if_launched_without_shell() -> bool {
    use windows_sys::Win32::System::Console::{GetConsoleProcessList, GetConsoleWindow};
    use windows_sys::Win32::UI::WindowsAndMessaging::{SW_HIDE, ShowWindow};

    // SAFETY: All called functions take no borrowed state; the buffer passed
    // to `GetConsoleProcessList` is a valid local array.
    unsafe {
        let hwnd = GetConsoleWindow();
        if hwnd.is_null() {
            // No Win32 console attached (e.g. mintty/MSYS2, GUI parent).
            return false;
        }

        let mut pids = [0u32; 4];
        let count = GetConsoleProcessList(pids.as_mut_ptr(), pids.len() as u32);
        if count != 1 {
            // 0 = error (keep console as fail-safe); >= 2 (or more than the
            // buffer holds) = console shared with a shell, keep it visible.
            return false;
        }

        // Sole owner, i.e. launched from Explorer/shortcut: hide the window
        // while staying attached so std handles remain valid.
        ShowWindow(hwnd, SW_HIDE) != 0
    }
}

/// Non-Windows stub: there is no Win32 console to hide.
#[cfg(not(windows))]
pub fn hide_console_if_launched_without_shell() -> bool {
    false
}
