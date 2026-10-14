use std::fs::File;
#[cfg(windows)]
use std::str::FromStr;

#[cfg(windows)]
use bdgm::game::ValidatedGame;
use hadris_udf::UdfVolume;
#[cfg(windows)]
use platform_dirs::AppDirs;
use tempfile::{TempDir, tempdir};

use crate::{
    cli::args::Args,
    core::{dump::extract_udf_dir, error::IoError},
};

#[cfg(windows)]
use crate::core::dirs::GameDirs;

#[cfg(windows)]
use crate::core::dump::{
    dump_disc_with_progress, raw_disc_has_manifest, read_manifest_from_raw_disc,
};
#[cfg(windows)]
use tempfile::NamedTempFile;

enum ImageState {
    MountedDirectory,
    Image,
    #[cfg(windows)]
    RawDisc,
}

impl ImageState {
    pub fn from(args: &Args) -> Self {
        #[cfg(windows)]
        if args.is_raw_disc() {
            return Self::RawDisc;
        }
        if args.image {
            Self::Image
        } else {
            Self::MountedDirectory
        }
    }
}

/// Raw-disc fast path: returns `Some` when the disc's `DISC.BDGM` can be
/// read with partial UDF seeks and its `id`+`version` is already installed,
/// letting the caller skip the full dump. Returns `None` on any preflight
/// failure or cache miss so the caller falls through to the full dump.
#[cfg(windows)]
fn try_raw_disc_cache_hit(args: &Args) -> Option<anyhow::Result<(Args, Option<TempDir>)>> {
    let drive = args.location.as_ref()?.to_string_lossy().into_owned();
    let runtime = args.runtime.clone();

    // 1. Fs-map-only probe: is DISC.BDGM even listed? No file data read.
    if !raw_disc_has_manifest(&drive).ok()? {
        return None;
    }
    // 2. Read just the manifest file's extents.
    let manifest = read_manifest_from_raw_disc(&drive).ok()?;
    let game = ValidatedGame::validate_manifest(&manifest).ok()?;
    let app_dirs = AppDirs::new(Some("bdgm-play"), true)?;
    if !GameDirs::from(&game, &app_dirs).install.try_exists().ok()? {
        return None;
    }

    // 3. Seed a tempdir with just BDGM/DISC.BDGM so the downstream
    // `read_manifest` + `install` (no-op) flow works unchanged.
    let extract_dir = tempdir().ok()?;
    std::fs::create_dir_all(extract_dir.path().join("BDGM")).ok()?;
    std::fs::write(extract_dir.path().join("BDGM").join("DISC.BDGM"), manifest).ok()?;
    println!("Game already installed, skipping disc dump.");
    Some(Ok((
        Args::new_imageless(Some(extract_dir.path().to_path_buf()), runtime),
        Some(extract_dir),
    )))
}

#[allow(unused_mut)]
pub fn resolve_image_args_with_progress(
    args: Args,
    mut on_progress: impl FnMut(u64, u64) + Send,
) -> anyhow::Result<(Args, Option<TempDir>)> {
    #[cfg(not(windows))]
    let _ = &mut on_progress;
    let image_state = ImageState::from(&args);

    let file = match image_state {
        ImageState::MountedDirectory => return Ok((args, None)),
        ImageState::Image => File::open(&args.location.ok_or(IoError::PathNone)?)?,
        #[cfg(windows)]
        ImageState::RawDisc => {
            // Fast path: read only DISC.BDGM off the disc (partial UDF
            // seeks) and skip the full dump when this id+version is
            // already installed. Any preflight failure falls through to
            // the full dump below.
            if let Some(cached) = try_raw_disc_cache_hit(&args) {
                return cached;
            }

            let dump_file = NamedTempFile::new()?;
            dump_disc_with_progress(
                &args.location.ok_or(IoError::PathNone)?.to_string_lossy(),
                &dump_file.path().to_string_lossy(),
                &mut on_progress,
            )?;

            File::open(dump_file.path())?
        }
    };

    let extract_dir = tempdir()?;
    let udf = UdfVolume::open(file)?;
    extract_udf_dir(&udf, &udf.root_dir()?, extract_dir.path())?;

    Ok((
        Args::new_imageless(Some(extract_dir.path().to_path_buf()), args.runtime),
        Some(extract_dir),
    ))
}
