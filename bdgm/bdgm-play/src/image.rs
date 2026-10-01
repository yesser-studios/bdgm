use std::fs::File;

use hadris_udf::UdfVolume;
use tempfile::{TempDir, tempdir};

use crate::{args::Args, dump::extract_udf_dir};

#[cfg(windows)]
use crate::dump::dump_disc;
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

pub fn resolve_image_args(args: Args) -> anyhow::Result<(Args, Option<TempDir>)> {
    let image_state = ImageState::from(&args);

    let file = match image_state {
        ImageState::MountedDirectory => return Ok((args, None)),
        ImageState::Image => File::open(&args.location)?,
        #[cfg(windows)]
        ImageState::RawDisc => {
            let dump_file = NamedTempFile::new()?;
            dump_disc(
                &args.location.to_string_lossy(),
                &dump_file.path().to_string_lossy(),
            )?;

            File::open(dump_file.path())?
        }
    };

    let extract_dir = tempdir()?;
    let udf = UdfVolume::open(file)?;
    extract_udf_dir(&udf, &udf.root_dir()?, extract_dir.path())?;

    Ok((
        Args::new_imageless(extract_dir.path().to_path_buf(), args.runtime),
        Some(extract_dir),
    ))
}
