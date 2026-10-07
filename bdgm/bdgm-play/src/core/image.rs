use std::fs::File;

use hadris_udf::UdfVolume;
use tempfile::{TempDir, tempdir};

use crate::{cli::args::Args, core::dump::extract_udf_dir, core::error::IoError};

#[cfg(windows)]
use crate::core::dump::dump_disc_with_progress;
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
