use std::{
    fs::File,
    io::{self, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use fs_extra::dir;

use crate::error::IoError;

pub fn acquire_lock(path: &Path, blocking: bool, verbose: bool) -> Result<File, IoError> {
    match path.parent() {
        Some(dir_path) => {
            if dir_path.exists() && !dir_path.is_dir() {
                return Err(IoError::IoError(io::Error::new(
                    io::ErrorKind::NotADirectory,
                    format!(
                        "Parent of {} (which was attempted to lock) exists but is not a directory",
                        path.display()
                    ),
                )));
            }
            dir::create_all(dir_path, false)?;
        }
        None => {} // Parent does not exist so locking will probably fail, but we'll still try
    }

    let file = get_file(&path.with_added_extension("lock"))?;
    if blocking && verbose {
        match file.try_lock() {
            Ok(()) => {}
            Err(e) => match e {
                std::fs::TryLockError::Error(error) => return Err(error.into()),
                std::fs::TryLockError::WouldBlock => {
                    println!("Waiting for lock on {}...", path.display());
                    file.lock()?;
                }
            },
        };
    } else if blocking && !verbose {
        file.lock()?;
    } else {
        file.try_lock()?;
    }
    Ok(file)
}

pub fn get_file(path: &Path) -> io::Result<File> {
    File::options()
        .write(true)
        .read(true)
        .create(true)
        .open(path)
}

pub fn get_part_file(path: &Path) -> io::Result<(File, PathBuf)> {
    let path = path.with_added_extension("part");
    Ok((get_file(&path)?, path))
}

pub fn truncate(file: &mut File) -> io::Result<()> {
    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;

    Ok(())
}
