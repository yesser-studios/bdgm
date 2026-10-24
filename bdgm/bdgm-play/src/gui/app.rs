use iced::{
    Theme,
    window::{self, icon},
};

use crate::args::Args;

use super::{
    state::{AppState, Status},
    update::update,
    view::view,
};

/// Runs the player window, retaining only the runtime override from `args`.
///
/// On Windows, attempts to hide a console owned solely by this process. Returns
/// Iced application errors and also displays them in a dialog if the console
/// was hidden.
pub fn run_gui(args: Args) -> iced::Result {
    let Args { runtime, .. } = args;
    let console_hidden = hide_console_if_no_shell();
    #[cfg(not(windows))]
    let _ = console_hidden;

    let result = iced::application(
        move || AppState {
            runtime: runtime.clone(),
            status: Status::Idle(None),
            disc_picker_discs: None,
        },
        update,
        view,
    )
    .theme(|_: &AppState| Theme::CatppuccinMocha)
    .title("BDGM Player")
    .window(window::Settings {
        icon: icon::from_file_data(include_bytes!("../../assets/icon.png"), None).ok(),
        size: iced::Size {
            width: 700.0,
            height: 500.0,
        },
        min_size: Some(iced::Size {
            width: 400.0,
            height: 300.0,
        }),
        ..Default::default()
    })
    .run();

    if let Err(error) = &result {
        eprintln!("Error while running app: {error}");

        // Explorer launches have no shell to print to, so surface the
        // failure in a dialog instead of losing it.
        #[cfg(windows)]
        if console_hidden {
            rfd::MessageDialog::new()
                .set_title("bdgm-play error")
                .set_description(format!("Error while running app: {error}"))
                .set_level(rfd::MessageLevel::Error)
                .show();
        }
    }

    result
}

/// Hide the console window if the GUI was launched outside of a shell.
///
/// Returns `true` when hiding a previously visible console owned solely by this
/// process. Returns `false` if there is no console, it is shared, its process list
/// cannot be queried, or the console was already hidden.
///
/// The console is hidden with `ShowWindow(SW_HIDE)` rather than detached with
/// `FreeConsole`: detaching destroys the console while stdin/stdout/stderr
/// handles still point at it, so later child-game spawns with inherited stdio
/// fail with OS error 50 (`ERROR_NOT_SUPPORTED`). Hiding keeps the console
/// attached (handles stay valid, games inherit the hidden console instead of
/// popping their own visible one) while making it invisible.
#[cfg(windows)]
fn hide_console_if_no_shell() -> bool {
    use windows_sys::Win32::System::Console::{GetConsoleProcessList, GetConsoleWindow};
    use windows_sys::Win32::UI::WindowsAndMessaging::{SW_HIDE, ShowWindow};

    // SAFETY: All called functions take no borrowed state; the buffer passed
    // to `GetConsoleProcessList` is a valid local array.
    unsafe {
        let window = GetConsoleWindow();
        if window.is_null() {
            // No Win32 console attached (e.g. mintty/MSYS2, GUI parent).
            return false;
        }

        let mut processes = [0u32; 4];
        let count = GetConsoleProcessList(processes.as_mut_ptr(), processes.len() as u32);
        if count != 1 {
            // 0 = error (keep console as fail-safe); >= 2 (or more than the
            // buffer holds) = console shared with a shell, keep it visible.
            return false;
        }

        // Sole owner, i.e. launched from Explorer/shortcut: hide the window
        // while staying attached so std handles remain valid.
        ShowWindow(window, SW_HIDE) != 0
    }
}

/// Non-Windows stub: there is no Win32 console to hide.
#[cfg(not(windows))]
fn hide_console_if_no_shell() -> bool {
    false
}
