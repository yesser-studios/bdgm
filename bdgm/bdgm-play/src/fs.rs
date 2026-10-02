use std::{
    fs::File,
    io::{self, Seek, SeekFrom},
    path::PathBuf,
};

pub fn acquire_lock(path: &PathBuf) -> io::Result<File> {
    let file = get_file(&path.with_added_extension("lock"))?;
    file.lock()?;
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
