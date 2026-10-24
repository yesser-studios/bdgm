use std::{
    fs::{File, TryLockError},
    io::{self, Seek, SeekFrom},
    path::Path,
};

use super::state::{State, StateHandler};

/// Opens or creates a sibling file with an added `.lock` extension and locks it exclusively.
///
/// When `blocking` is false, contention returns `WouldBlock`. Otherwise, reports
/// `WaitingForLock` while waiting and reverts that state when the lock attempt
/// finishes. The returned file holds the lock until released or dropped.
///
/// Parent directories must exist. File opening and locking errors propagate.
pub(super) fn acquire_lock(
    path: &Path,
    blocking: bool,
    mut on_state: Option<&mut dyn StateHandler>,
) -> io::Result<File> {
    let lock_path = path.with_added_extension("lock");
    let file = open_file(&lock_path)?;

    if !blocking {
        file.try_lock().map_err(try_lock_error)?;
        return Ok(file);
    }

    match file.try_lock() {
        Ok(()) => {}
        Err(TryLockError::Error(error)) => return Err(error),
        Err(TryLockError::WouldBlock) => {
            if let Some(handler) = &mut on_state {
                handler.change_state(State::WaitingForLock(lock_path));
            }

            let result = file.lock();

            if let Some(handler) = &mut on_state {
                handler.revert_state();
            }

            result?;
        }
    }

    Ok(file)
}

/// Opens a file for reading and writing, creating it if missing and preserving existing contents.
///
/// Parent directories must exist; file opening errors propagate.
pub(super) fn open_file(path: &Path) -> io::Result<File> {
    File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
}

/// Opens or creates the sibling with an added `.part` extension, preserving its contents.
///
/// Propagates file opening errors, including a missing parent directory.
pub(super) fn open_part_file(path: &Path) -> io::Result<File> {
    open_file(&path.with_added_extension("part"))
}

/// Empties the file and rewinds its cursor, propagating truncation or seek errors.
pub(super) fn truncate(file: &mut File) -> io::Result<()> {
    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;

    Ok(())
}

fn try_lock_error(error: TryLockError) -> io::Error {
    match error {
        TryLockError::Error(error) => error,
        TryLockError::WouldBlock => {
            io::Error::new(io::ErrorKind::WouldBlock, "the lock is already held")
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::Read,
        io::Write,
        sync::{Arc, Mutex, mpsc},
        thread,
        time::Duration,
    };

    use tempfile::tempdir;

    use super::*;
    use crate::core::test_utils::RecordingStateHandler;

    struct NotifyingStateHandler {
        inner: RecordingStateHandler,
        notify: Arc<Mutex<mpsc::Sender<()>>>,
    }

    impl StateHandler for NotifyingStateHandler {
        fn change_state(&mut self, state: State) {
            self.inner.change_state(state);

            if let Ok(sender) = self.notify.lock() {
                let _ = sender.send(());
            }
        }

        fn revert_state(&mut self) {
            self.inner.revert_state();
        }
    }

    #[test]
    fn open_file_preserves_existing_contents() {
        let temp = tempdir().expect("create temporary directory");
        let path = temp.path().join("file");
        fs::write(&path, b"hello").expect("write initial contents");

        let mut file = open_file(&path).expect("open file");
        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("read file");

        assert_eq!(contents, "hello");
    }

    #[test]
    fn open_part_file_uses_the_part_extension() {
        let temp = tempdir().expect("create temporary directory");
        let path = temp.path().join("install");

        let _file = open_part_file(&path).expect("open part file");

        assert!(path.with_added_extension("part").is_file());
    }

    #[test]
    fn truncate_clears_contents_and_rewinds() {
        let temp = tempdir().expect("create temporary directory");
        let path = temp.path().join("file");
        fs::write(&path, b"hello").expect("write initial contents");

        let mut file = open_file(&path).expect("open file");
        truncate(&mut file).expect("truncate file");
        file.write_all(b"x").expect("write after truncate");
        drop(file);

        assert_eq!(fs::read_to_string(&path).expect("read file"), "x");
    }

    #[test]
    fn nonblocking_lock_conflicts_with_an_existing_lock() {
        let temp = tempdir().expect("create temporary directory");
        let path = temp.path().join("resource");

        let first = acquire_lock(&path, false, None).expect("acquire first lock");
        let error = acquire_lock(&path, false, None).expect_err("second lock should fail");
        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);

        drop(first);
        assert!(acquire_lock(&path, false, None).is_ok());
    }

    #[test]
    fn blocking_lock_reports_waiting_state_and_reverts() {
        let temp = tempdir().expect("create temporary directory");
        let path = temp.path().join("resource");
        let expected_lock = path.with_added_extension("lock");
        let first = acquire_lock(&path, false, None).expect("acquire first lock");

        let (waiting_tx, waiting_rx) = mpsc::channel();
        let notify = Arc::new(Mutex::new(waiting_tx));
        let (finished_tx, finished_rx) = mpsc::channel();
        let thread_path = path.clone();
        let handle = thread::spawn(move || {
            let mut state = NotifyingStateHandler {
                inner: RecordingStateHandler::new(),
                notify,
            };
            let lock = acquire_lock(&thread_path, true, Some(&mut state)).expect("acquire lock");
            drop(lock);
            finished_tx.send(state.inner).expect("send recorded state");
        });

        waiting_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("waiting thread did not block on the lock");
        drop(first);

        let state = finished_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("waiting thread did not finish");
        handle.join().expect("join waiting thread");

        assert!(matches!(
            state.states.first(),
            Some(State::WaitingForLock(lock_path)) if lock_path == &expected_lock
        ));
        assert_eq!(state.reverts, 1);
    }
}
