use iced::Task;

use super::super::{message::Message, state::AppState};

#[cfg(windows)]
const SECTOR_SIZE: u64 = 2048;

pub(super) fn handle_dump_progress(state: &mut AppState, done: u64, total: u64) -> Task<Message> {
    // Ignore stale progress (e.g. after the game already started/failed).
    if !matches!(state.phase, super::super::state::Phase::Starting) {
        return Task::none();
    }
    state.dump_progress = Some((done, total));
    #[cfg(windows)]
    {
        let mib_done = done as f64 * SECTOR_SIZE as f64 / 1024.0 / 1024.0;
        let mib_total = total as f64 * SECTOR_SIZE as f64 / 1024.0 / 1024.0;
        let pct = if total > 0 {
            done as f64 * 100.0 / total as f64
        } else {
            0.0
        };
        state.status = format!("Reading disc: {mib_done:.1}/{mib_total:.1} MiB ({pct:.1}%)");
    }
    #[cfg(not(windows))]
    {
        state.status = format!("Reading disc: {done}/{total} sectors");
    }
    Task::none()
}
