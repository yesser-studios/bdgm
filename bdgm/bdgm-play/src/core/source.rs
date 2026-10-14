use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub enum Source {
    MountedDir(PathBuf),
    // Image(PathBuf),
    // RawDisc(PathBuf),
}
impl Source {
    pub fn resolve(
        self,
        mut on_progress: impl FnMut(u64, u64) + Send,
    ) -> anyhow::Result<ResolvedSource> {
        match self {
            Source::MountedDir(path) => Ok(ResolvedSource {
                root_path: path,
                _temp_dir: None,
            }),
            // let file = match launch_request.source {
            //     ImageState::MountedDirectory => return Ok((launch_request, None)),
            //     ImageState::Image => File::open(&launch_request.location.ok_or(IoError::PathNone)?)?,
            //     #[cfg(windows)]
            //     ImageState::RawDisc => {
            //         // Fast path: read only DISC.BDGM off the disc (partial UDF
            //         // seeks) and skip the full dump when this id+version is
            //         // already installed. Any preflight failure falls through to
            //         // the full dump below.
            //         if let Some(cached) = try_raw_disc_cache_hit(&launch_request) {
            //             return cached;
            //         }
            //
            //         let dump_file = NamedTempFile::new()?;
            //         dump_disc_with_progress(
            //             &launch_request
            //                 .location
            //                 .ok_or(IoError::PathNone)?
            //                 .to_string_lossy(),
            //             &dump_file.path().to_string_lossy(),
            //             &mut on_progress,
            //         )?;
            //
            //         File::open(dump_file.path())?
            //     }
            // };
            // let extract_dir = tempdir()?;
            // let udf = UdfVolume::open(file)?;
            // extract_udf_dir(&udf, &udf.root_dir()?, extract_dir.path())?;
        }
    }
}

pub struct ResolvedSource {
    root_path: PathBuf,
    _temp_dir: Option<TempDir>,
}
impl ResolvedSource {
    pub fn root(&self) -> &Path {
        &self.root_path
    }
}

pub struct LaunchRequest {
    pub source: Source,
    pub runtime_path: Option<PathBuf>,
}
impl LaunchRequest {
    pub fn new(source: Source, runtime: Option<PathBuf>) -> Self {
        Self {
            source,
            runtime_path: runtime,
        }
    }
}
