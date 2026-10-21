use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

pub use hadris_udf::UdfRevision;
use hadris_udf::write::{SimpleDir, SimpleFile, UdfWriteOptions, UdfWriter};

use crate::{
    error::{BDGMError, EntryError},
    game::ValidatedGame,
};

/// A single filesystem file or directory entry.
#[derive(Debug, Clone)]
pub enum Entry {
    File { source: PathBuf, path: PathBuf },
    Directory { path: PathBuf, children: Vec<Entry> },
}

impl Entry {
    /// Scans a directory and returns an `Entry` of the root directory, including all descendants.
    pub fn scan_dir(root: impl AsRef<Path>) -> anyhow::Result<Entry> {
        let root = root.as_ref();
        let mut children: Vec<Entry> = Vec::new();

        for entry in fs::read_dir(root)? {
            let entry = entry?;
            let path = entry.path();

            let relative_path = path.strip_prefix(root)?.to_path_buf();

            if path.is_dir() {
                children.push(Entry::scan_entry(&path, relative_path)?);
            } else {
                children.push(Entry::File {
                    source: path,
                    path: relative_path,
                });
            }
        }

        Ok(Entry::Directory {
            path: PathBuf::new(),
            children,
        })
    }

    /// Scans a single file or directory and creates an `Entry` containing all children
    fn scan_entry(source: &Path, path: PathBuf) -> io::Result<Entry> {
        if source.is_dir() {
            let mut children = Vec::new();

            for entry in fs::read_dir(source)? {
                let entry = entry?;
                let child_source = entry.path();

                let child_name = child_source.file_name().unwrap();

                children.push(Entry::scan_entry(&child_source, child_name.into())?);
            }

            Ok(Entry::Directory { path, children })
        } else {
            Ok(Entry::File {
                source: source.to_path_buf(),
                path: path.file_name().unwrap().into(),
            })
        }
    }

    /// Creates a `hadris_udf::sync::SimpleDir` from a directory `Entry`.
    pub fn to_simple_dir(&self) -> anyhow::Result<SimpleDir> {
        match self {
            Entry::File { source: _, path: _ } => Result::Err(EntryError::EntryIsFile.into()),
            Entry::Directory { path, children } => {
                let mut root = SimpleDir::new(path.to_string_lossy());
                for entry in children {
                    match entry {
                        Entry::File { source, path } => {
                            let data = fs::read(source)?;
                            root.add_file(SimpleFile::new(path.to_string_lossy(), data));
                        }
                        Entry::Directory { path, children: _ } => {
                            let mut child = entry.to_simple_dir()?;
                            child.name = path.to_string_lossy().into_owned();
                            root.add_dir(child);
                        }
                    }
                }
                Ok(root)
            }
        }
    }

    /// Writes a directory `Entry` including its children into a UDF image file.
    pub fn write_udf(&self, output: PathBuf, revision: UdfRevision) -> anyhow::Result<()> {
        match self {
            Entry::File { source: _, path: _ } => Err(EntryError::EntryIsFile.into()),
            Entry::Directory { path: _, children } => {
                let mut bdgm_dir_children = Vec::new();
                children
                    .iter()
                    .find(|it| match it {
                        Entry::Directory { path, children } if path == "BDGM" => {
                            bdgm_dir_children = children.to_vec();
                            true
                        }
                        _ => false,
                    })
                    .ok_or(BDGMError::BDGMDirectoryMissing)?;
                let mut manifest_path = PathBuf::new();
                bdgm_dir_children
                    .iter()
                    .find(|it| match it {
                        Entry::File { source, path } if path == "DISC.BDGM" => {
                            manifest_path = source.to_path_buf();
                            true
                        }
                        _ => false,
                    })
                    .ok_or(BDGMError::DiscFileMissing)?;

                let mut manifest = fs::File::open(manifest_path)?;
                let mut contents = String::new();
                manifest.read_to_string(&mut contents)?;

                let game = ValidatedGame::validate_manifest(&contents)?;
                let executable = game.executable();

                let mut app_dir_children = Vec::new();
                bdgm_dir_children
                    .iter()
                    .find(|it| match it {
                        Entry::Directory { path, children } if path == "APP" => {
                            app_dir_children = children.to_vec();
                            true
                        }
                        _ => false,
                    })
                    .ok_or(BDGMError::AppDirectoryMissing)?;
                app_dir_children
                    .iter()
                    .find(|it| match it {
                        Entry::File { source: _, path } => *path == *executable,
                        _ => false,
                    })
                    .ok_or(BDGMError::ExecutableMissing(
                        executable.to_string_lossy().into_owned(),
                    ))?;

                write_simple_dir(
                    &self.to_simple_dir()?,
                    game.name().to_string(),
                    output,
                    revision,
                )
            }
        }
    }
}

/// Writes a `hadris_udf::sync::SimpleDir` into a UDF image file.
pub fn write_simple_dir(
    root: &SimpleDir,
    name: String,
    output: PathBuf,
    revision: UdfRevision,
) -> anyhow::Result<()> {
    let options = UdfWriteOptions {
        volume_id: name,
        revision,
        ..UdfWriteOptions::default()
    };

    let output_file = fs::File::create(output)?;
    let output = UdfWriter::create(output_file, root, options)?;
    println!(
        "Wrote {} 2048-byte sectors, total size: {} B",
        output.sectors_written,
        output.sectors_written as u64 * 2048
    );

    Ok(())
}

#[cfg(test)]
mod test {
    use std::{fs, path::PathBuf};

    use hadris_udf::{UdfRevision, UdfVolume};
    use tempfile::tempdir;

    use crate::disc::Entry;

    #[test]
    fn sample_dir_written() {
        let disc = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../disc");
        let temp = tempdir().expect("create temporary directory");
        let output = temp.path().join("result.udf");

        let entry = Entry::scan_dir(&disc).expect("scan sample disc");
        entry
            .write_udf(output.clone(), UdfRevision::V2_50)
            .expect("write sample disc");

        assert!(
            fs::metadata(&output).expect("read output metadata").len() > 0,
            "the written image should not be empty"
        );

        let file = fs::File::open(&output).expect("open written image");
        let volume = UdfVolume::open(file).expect("open written UDF image");
        let root = volume.root_dir().expect("read UDF root");
        assert!(
            root.find("BDGM").is_some(),
            "the written image should contain the BDGM directory"
        );
    }
}
