use std::{
    fs::File,
    io::{self, Seek, SeekFrom},
    path::PathBuf,
};

use fs_extra::dir;

use crate::error::IoError;

pub fn acquire_lock(path: &PathBuf, blocking: bool) -> Result<File, IoError> {
    match path.parent() {
        Some(dir_path) => {
            if dir_path.is_dir() {
                dir::create_all(dir_path, false)?;
            }
        }
        None => todo!(),
    }

    let file = get_file(&path.with_added_extension("lock"))?;
    if blocking {
        file.lock()?;
    } else {
        file.try_lock()?;
    }
    Ok(file)
}

pub fn get_file(path: &PathBuf) -> io::Result<File> {
    File::options()
        .write(true)
        .read(true)
        .create(true)
        .open(path)
}

pub fn get_part_file(path: &PathBuf) -> io::Result<(File, PathBuf)> {
    let path = path.with_added_extension("part");
    Ok((get_file(&path)?, path))
}

pub fn truncate(file: &mut File) -> io::Result<()> {
    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;

    Ok(())
}
