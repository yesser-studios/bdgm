use std::{
    fs::{File, TryLockError},
    io::{self, Seek, SeekFrom},
    path::Path,
};

use super::state::{State, StateHandler};

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

pub(super) fn open_file(path: &Path) -> io::Result<File> {
    File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
}

pub(super) fn open_part_file(path: &Path) -> io::Result<File> {
    open_file(&path.with_added_extension("part"))
}

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
