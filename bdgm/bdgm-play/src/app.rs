use std::{collections::HashMap, fs::File, io::Read, process::Command, string::String};

use bdgm::{error::BDGMError, game::Game};
use clap::Parser;
use fs_extra::dir::{self, CopyOptions};
use hadris_udf::UdfVolume;
use platform_dirs::AppDirs;
use tempfile::tempdir;

use crate::{args::Args, dump::extract_udf_dir, error::AppError};

#[cfg(windows)]
use crate::dump::dump_disc;
#[cfg(windows)]
use tempfile::NamedTempFile;

pub(crate) fn run() -> anyhow::Result<()> {
    let app_dirs = AppDirs::new(Some("bdgm-play"), true).ok_or(AppError::NoAppDirs)?;
    let extract_dir = tempdir()?;

    let args = {
        let args = Args::parse();
        println!("Playing {}", args.location.to_string_lossy());

        #[allow(unused_assignments)]
        let mut raw_disc = false;
        #[cfg(windows)]
        {
            raw_disc = args.raw_disc;
        }

        if args.image || raw_disc {
            let file = if args.image {
                File::open(&args.location)?
            } else {
                #[cfg(windows)]
                {
                    let dump_file = NamedTempFile::new()?;
                    dump_disc(
                        &args.location.to_string_lossy(),
                        &dump_file.path().to_string_lossy(),
                    )?;

                    File::open(dump_file.path())?
                }
                #[cfg(not(windows))]
                {
                    File::open("")?
                }
            };

            let udf = UdfVolume::open(file)?;
            extract_udf_dir(&udf, &udf.root_dir()?, extract_dir.path())?;
            #[cfg(windows)]
            {
                Args {
                    location: extract_dir.path().to_path_buf(),
                    image: false,
                    raw_disc: false,
                    runtime: None,
                }
            }
            #[cfg(not(windows))]
            {
                Args {
                    location: extract_dir.path().to_path_buf(),
                    image: false,
                    runtime: None,
                }
            }
        } else {
            args
        }
    };

    let manifest_path = args.location.join("BDGM").join("DISC.BDGM");

    if !manifest_path.try_exists()? {
        return Err(BDGMError::DiscFileMissing.into());
    }

    let mut manifest = File::open(manifest_path)?;
    let mut contents = String::new();

    println!("Reading manifest...");
    manifest.read_to_string(&mut contents)?;
    let game = Game::from_str(&contents)?;

    let app_dir_path = args.location.join("BDGM").join("APP");
    let executable_path = app_dir_path.join(&game.executable);

    if !executable_path.try_exists()? {
        return Err(BDGMError::ExecutableMissing.into());
    }

    let game_dir = app_dirs.data_dir.join(&game.id);
    let cache_dir = app_dirs.cache_dir.join(&game.id);
    let data_dir = game_dir.join("data");
    let install_dir = game_dir.join("app").join(game.version);

    if !install_dir.try_exists()? {
        println!("Copying files...");
        dir::create_all(&install_dir, false)?;
        dir::copy(
            &app_dir_path,
            &install_dir,
            &CopyOptions::new().overwrite(true).content_only(true),
        )?;
        println!("Copied!");
    }

    let mut envvars = HashMap::new();
    envvars.insert("BDGM_DATA", data_dir.to_string_lossy().into_owned());
    envvars.insert("XDG_DATA_DIRS", data_dir.to_string_lossy().into_owned());
    envvars.insert("BDGM_CACHE", cache_dir.to_string_lossy().into_owned());
    envvars.insert("BDGM_APP", app_dir_path.to_string_lossy().into_owned());
    envvars.insert("BDGM_DISC", args.location.to_string_lossy().into_owned());
    envvars.insert("BDGM_VERSION", "1.0".to_string());

    let runtime = args.runtime.map(|x| x.to_string_lossy().to_string());

    let status = match game.runtime {
        bdgm::runtime::Runtime::Java => Command::new(runtime.as_deref().unwrap_or("java"))
            .args(game.runtime_args)
            .arg("--jar")
            .arg(install_dir.join(game.executable))
            .arg("--")
            .args(game.args)
            .envs(envvars)
            .current_dir(&install_dir)
            .status()?,
        bdgm::runtime::Runtime::Dotnet => Command::new(runtime.as_deref().unwrap_or("dotnet"))
            .args(game.runtime_args)
            .arg(install_dir.join(game.executable))
            .arg("--")
            .args(game.args)
            .envs(envvars)
            .current_dir(&install_dir)
            .status()?,
        bdgm::runtime::Runtime::Python => Command::new(runtime.as_deref().unwrap_or("python"))
            .args(game.runtime_args)
            .arg(install_dir.join(game.executable))
            .arg("--")
            .args(game.args)
            .envs(envvars)
            .current_dir(&install_dir)
            .status()?,
        bdgm::runtime::Runtime::Windows => {
            if cfg!(target_os = "windows") {
                Command::new(install_dir.join(game.executable))
                    .args(game.args)
                    .envs(envvars)
                    .current_dir(&install_dir)
                    .status()?
            } else {
                Command::new(runtime.as_deref().unwrap_or("wine"))
                    .args(game.runtime_args)
                    .arg(install_dir.join(game.executable))
                    .args(game.args)
                    .envs(envvars)
                    .env("WINEPREFIX", game_dir.join("wineprefix"))
                    .current_dir(&install_dir)
                    .status()?
            }
        }
    };
    if status.success() {
        eprintln!("Your game crashed: {status}");
        eprintln!("Setting a runtime with `--runtime /path/to/runtime` may fix your issue.");
    }

    Ok(())
}
