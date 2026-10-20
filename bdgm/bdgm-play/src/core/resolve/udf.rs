use std::{fs, io, path::Path, str::FromStr};

use bdgm::game::Game;
use hadris_udf::{UdfDir, UdfVolume};
use tempfile::tempdir;

use crate::core::{
    error::{AppError, ExtractError, Result},
    request::ExtractedSource,
    state::{State, StateHandler},
};

pub(super) fn open_udf_volume<T: io::Read + io::Seek>(reader: T) -> Result<UdfVolume<T>> {
    UdfVolume::open(reader).map_err(udf_error)
}

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
