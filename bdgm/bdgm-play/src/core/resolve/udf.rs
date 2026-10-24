use std::{fs, io, path::Path, str::FromStr};

use bdgm::game::Game;
use hadris_udf::{UdfDir, UdfVolume};
use tempfile::tempdir;

use crate::core::{
    error::{AppError, ExtractError, Result},
    request::ExtractedSource,
    state::{State, StateHandler},
};

/// Opens a UDF volume, converting UDF errors to extraction I/O errors with kind `InvalidData`.
pub(super) fn open_udf_volume<T: io::Read + io::Seek>(reader: T) -> Result<UdfVolume<T>> {
    UdfVolume::open(reader).map_err(udf_error)
}

/// Probes `BDGM/DISC.BDGM` without validating its field values.
///
/// Missing entries, incorrect entry kinds, invalid UTF-8, and parse failures
/// return `None`. UDF directory and file read failures return extraction errors.
pub(super) fn get_udf_manifest(udf: &UdfVolume<impl io::Read + io::Seek>) -> Result<Option<Game>> {
    let root = udf.root_dir().map_err(udf_error)?;

    let Some(bdgm_entry) = root.find("BDGM") else {
        return Ok(None);
    };
    if !bdgm_entry.is_dir() {
        return Ok(None);
    }

    let bdgm_dir = udf.read_directory(&bdgm_entry.icb).map_err(udf_error)?;
    let Some(manifest_entry) = bdgm_dir.find("DISC.BDGM") else {
        return Ok(None);
    };
    if !manifest_entry.is_file() {
        return Ok(None);
    }

    let bytes = udf.read_file(manifest_entry).map_err(udf_error)?;
    let Ok(manifest) = String::from_utf8(bytes) else {
        return Ok(None);
    };

    match Game::from_str(&manifest) {
        Ok(game) => Ok(Some(game)),
        Err(_) => Ok(None),
    }
}

/// Extracts a UDF volume into an owned temporary directory, reporting copied bytes.
///
/// Skips names rejected by `is_safe_name`. Temporary storage creation, UDF reads,
/// and filesystem write errors propagate as extraction errors.
pub(super) fn extract(
    udf: &UdfVolume<impl io::Read + io::Seek>,
    on_state: &mut dyn StateHandler,
) -> Result<ExtractedSource> {
    let temp_dir = tempdir().map_err(ExtractError::IoError)?;
    let root = udf.root_dir().map_err(udf_error)?;
    let total_size = total_size(udf, &root)?;
    let mut current_size = 0;

    extract_dir(
        udf,
        &root,
        temp_dir.path(),
        &mut current_size,
        &total_size,
        on_state,
    )?;

    Ok(ExtractedSource {
        path: temp_dir.path().to_path_buf(),
        _tempdir: Some(temp_dir),
    })
}

/// Recursively writes directory contents, skipping parent entries and unsafe names.
///
/// Updates `current_size` and reports `(current_size, total_size)` in bytes after
/// each file is written. UDF and filesystem failures return extraction errors
/// and can leave already written files in `output_path`.
pub(super) fn extract_dir(
    udf: &UdfVolume<impl io::Read + io::Seek>,
    dir: &UdfDir,
    output_path: &Path,
    current_size: &mut u64,
    total_size: &u64,
    on_state: &mut dyn StateHandler,
) -> Result<()> {
    for entry in dir.entries() {
        if entry.is_parent() || !is_safe_name(entry.name()) {
            continue;
        }

        let output = output_path.join(entry.name());

        if entry.is_dir() {
            fs::create_dir_all(&output).map_err(ExtractError::IoError)?;
            let child = udf.read_directory(&entry.icb).map_err(udf_error)?;
            extract_dir(udf, &child, &output, current_size, total_size, on_state)?;
        } else {
            let bytes = udf.read_file(entry).map_err(udf_error)?;
            fs::write(&output, &bytes).map_err(ExtractError::IoError)?;

            *current_size += bytes.len() as u64;
            on_state.change_state(State::Extracting(Some((*current_size, *total_size))));
        }
    }

    Ok(())
}

/// Sums recorded file sizes in bytes beneath entries accepted by `is_safe_name`.
///
/// Returns extraction errors if a UDF directory cannot be read.
fn total_size(udf: &UdfVolume<impl io::Read + io::Seek>, dir: &UdfDir) -> Result<u64> {
    let mut total = 0;

    for entry in dir.entries() {
        if !is_safe_name(entry.name()) {
            continue;
        }

        if entry.is_dir() {
            let child = udf.read_directory(&entry.icb).map_err(udf_error)?;
            total += total_size(udf, &child)?;
        } else {
            total += entry.size;
        }
    }

    Ok(total)
}

/// Checks whether the host path parser yields exactly one normal component for the name.
fn is_safe_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let mut components = Path::new(name).components();
    matches!(components.next(), Some(std::path::Component::Normal(_)))
        && components.next().is_none()
}

fn udf_error(error: hadris_udf::Error) -> AppError {
    ExtractError::IoError(io::Error::new(
        io::ErrorKind::InvalidData,
        error.to_string(),
    ))
    .into()
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn safe_names_only_allow_a_single_normal_component() {
        for name in ["file.txt", "dir-file", "a.b", "game"] {
            assert!(is_safe_name(name), "{name} should be safe");
        }

        for name in ["", ".", "..", "/absolute", "dir/file", "a/../b"] {
            assert!(!is_safe_name(name), "{name} should not be safe");
        }
    }

    #[test]
    fn open_udf_volume_rejects_non_udf_data() {
        let temp = tempdir().expect("create temporary directory");
        let path = temp.path().join("not-udf");
        fs::write(&path, b"not a udf volume").expect("write invalid UDF data");
        let file = File::open(path).expect("open invalid UDF data");

        let result = open_udf_volume(file);

        assert!(matches!(result, Err(AppError::ExtractError(_))));
    }
}
