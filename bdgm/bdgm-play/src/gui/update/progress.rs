use iced::Task;

use super::super::{
    message::Message,
    state::{AppState, Phase},
};

pub(super) fn handle_dump_progress(state: &mut AppState, done: u64, total: u64) -> Task<Message> {
    // Ignore stale progress (e.g. after the game already started/failed).
    // Progress values are file/disc **bytes** (`(done, total)`), shared by
    // the raw-disc dump and the direct-from-disc install paths.
    if !matches!(state.phase, Phase::Starting) {
        return Task::none();
    }
    state.dump_progress = Some((done, total));
    let mib_done = done as f64 / 1024.0 / 1024.0;
    let mib_total = total as f64 / 1024.0 / 1024.0;
    let pct = if total > 0 {
        done as f64 * 100.0 / total as f64
    } else {
        0.0
    };
    state.status = format!("Reading disc: {mib_done:.1}/{mib_total:.1} MiB ({pct:.1}%)");
    Task::none()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn dump_progress_reports_mib_for_byte_values() {
        let mut state = AppState {
            phase: Phase::Starting,
            ..Default::default()
        };
        handle_dump_progress(&mut state, 106_453_103, 106_453_103);
        assert_eq!(state.dump_progress, Some((106_453_103, 106_453_103)));
        assert!(
            state.status.contains("101.5/101.5 MiB (100.0%)"),
            "status was: {}",
            state.status
        );
    }
}
