use std::fs::File;
#[cfg(windows)]
use std::str::FromStr;

#[cfg(windows)]
use bdgm::game::{Game, ValidatedGame};
use hadris_udf::UdfVolume;
#[cfg(windows)]
use platform_dirs::AppDirs;
use tempfile::{TempDir, tempdir};

use crate::{
    cli::args::Args,
    core::{
        dump::extract_udf_dir,
        error::IoError,
    },
};

#[cfg(windows)]
use crate::core::dirs::GameDirs;

#[cfg(windows)]
use crate::core::dump::{
    dump_disc_with_progress, raw_disc_has_manifest, read_manifest_from_raw_disc,
};
#[cfg(windows)]
use crate::core::install::install_from_raw_disc;
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

/// Seeds a tempdir with just `BDGM/DISC.BDGM` so the downstream
/// `read_manifest` + `install` (no-op) flow works unchanged.
#[cfg(windows)]
fn seed_manifest_tempdir(
    manifest: String,
    runtime: Option<std::path::PathBuf>,
) -> anyhow::Result<(Args, Option<TempDir>)> {
    let extract_dir = tempdir()?;
    std::fs::create_dir_all(extract_dir.path().join("BDGM"))?;
    std::fs::write(extract_dir.path().join("BDGM").join("DISC.BDGM"), manifest)?;
    Ok((
        Args::new_imageless(Some(extract_dir.path().to_path_buf()), runtime),
        Some(extract_dir),
    ))
}

/// Raw-disc fast path: reads only `DISC.BDGM` with partial UDF seeks. On a
/// cache hit the dump is skipped; on a miss the game is installed straight
/// off the disc (`APP` subtree only) instead of dumping + fully extracting.
/// Returns `Some` once the game is installed either way; returns `None` when
/// the disc can't be pre-read (fall back to the full dump below).
/// Install errors propagate — a disc that fails partial reads will fail the
/// dump's sector reads too.
#[cfg(windows)]
fn try_raw_disc_direct(args: &Args) -> Option<anyhow::Result<(Args, Option<TempDir>)>> {
    let drive = args.location.as_ref()?.to_string_lossy().into_owned();
    let runtime = args.runtime.clone();

    // 1. Fs-map-only probe: is DISC.BDGM even listed? No file data read.
    if !raw_disc_has_manifest(&drive).ok()? {
        return None;
    }
    // 2. Read just the manifest file's extents.
    let manifest = read_manifest_from_raw_disc(&drive).ok()?;
    let game = ValidatedGame::validate(Game::from_str(&manifest).ok()?).ok()?;
    let app_dirs = AppDirs::new(Some("bdgm-play"), true)?;

    // 3a. Cache hit: skip everything.
    if GameDirs::from(&game, &app_dirs)
        .install
        .try_exists()
        .ok()?
    {
        println!("Game already installed, skipping disc dump.");
        return Some(seed_manifest_tempdir(manifest, runtime));
    }

    // 3b. Cache miss: install APP straight off the disc.
    println!("Installing game from disc...");
    if let Err(e) = install_from_raw_disc(&game, &app_dirs, &drive) {
        return Some(Err(e));
    }
    Some(seed_manifest_tempdir(manifest, runtime))
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
            // Fast path: install straight off the disc (or skip entirely
            // on cache hit) with partial UDF reads. Unreadable discs fall
            // through to the full dump below.
            if let Some(direct) = try_raw_disc_direct(&args) {
                return direct;
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
