use std::{collections::HashMap, fs::File, io::Read, process::Command, string::String};

use bdgm::{
    error::BDGMError,
    game::{Game, ValidatedGame},
};
use clap::Parser;
use fs_extra::dir::{self, CopyOptions};
use hadris_udf::UdfVolume;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use platform_dirs::AppDirs;
use tempfile::tempdir;

use crate::{
    args::Args,
    dump::extract_udf_dir,
    error::AppError,
    server::{create_listener, get_portlist_file, load_ports, save_ports, serve},
};

#[cfg(windows)]
use crate::dump::dump_disc;
#[cfg(windows)]
use tempfile::NamedTempFile;

pub(crate) async fn run() -> anyhow::Result<()> {
    let app_dirs = AppDirs::new(Some("bdgm-play"), true).ok_or(AppError::NoAppDirs)?;
    let extract_dir = tempdir()?;

    let args = {
        let args = Args::parse();
        println!("Playing {}", args.location.to_string_lossy());

        #[allow(unused_assignments)]
        #[allow(unused_mut)]
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
    let game = ValidatedGame::validate(game)?;

    let app_dir_path = args.location.join("BDGM").join("APP");
    let executable_str = game.executable();
    let executable_path = app_dir_path.join(&executable_str);

    if !executable_path.try_exists()? {
        return Err(AppError::InvalidGameFile(BDGMError::ExecutableMissing(
            executable_path.to_string_lossy().into_owned(),
        ))
        .into());
    }

    let id = game.id();
    let game_dir = app_dirs.data_dir.join(&id);
    let cache_dir = app_dirs.cache_dir.join(&id);
    let data_dir = game_dir.join("data");
    let install_dir = game_dir.join("app").join(game.version());
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
    envvars.insert("BDGM_VERSION", game.bdgm_version().to_string());

    let runtime_path = args.runtime.map(|x| x.to_string_lossy().to_string());

    let runtime = game.runtime();

    let status = match runtime {
        bdgm::runtime::Runtime::Java => Command::new(runtime_path.as_deref().unwrap_or("java"))
            .args(game.runtime_args())
            .arg("--jar")
            .arg(install_dir.join(executable_str))
            .arg("--")
            .args(game.args())
            .envs(envvars)
            .current_dir(&install_dir)
            .status()?,
        bdgm::runtime::Runtime::Dotnet => Command::new(runtime_path.as_deref().unwrap_or("dotnet"))
            .args(game.runtime_args())
            .arg(install_dir.join(executable_str))
            .arg("--")
            .args(game.args())
            .envs(envvars)
            .current_dir(&install_dir)
            .status()?,
        bdgm::runtime::Runtime::Python => Command::new(runtime_path.as_deref().unwrap_or("python"))
            .args(game.runtime_args())
            .arg(install_dir.join(executable_str))
            .arg("--")
            .args(game.args())
            .envs(envvars)
            .current_dir(&install_dir)
            .status()?,
        bdgm::runtime::Runtime::HTML => {
            println!("Reading saved ports...");
            let mut file = get_portlist_file(&app_dirs.data_dir)?;
            let mut ports = load_ports(&mut file)?;
            let port = ports.get_by_left(game.id());

            let listener = match port {
                Some(port) => create_listener(Some(*port)).await?,
                None => {
                    let mut listener = create_listener(None).await?;
                    let mut port = listener.local_addr()?.port();

                    let mut tries = 0;
                    while ports.get_by_right(&&port).is_some() {
                        listener = create_listener(None).await?;
                        port = listener.local_addr()?.port();

                        tries += 1;
                        if tries >= 5000 {
                            return Err(AppError::CouldNotFindUnclaimedPort.into());
                        }
                    }

                    ports.insert(game.id().to_string(), listener.local_addr()?.port());
                    println!("Persisting port {}...", listener.local_addr()?.port());
                    save_ports(ports, &mut file)?;
                    listener
                }
            };

            let executable = executable_str.to_string_lossy();
            let encoded = utf8_percent_encode(&executable, &ENCODE_SET);

            let address = format!("http://{}/{}", listener.local_addr()?, encoded);
            println!("Opening {address}");
            webbrowser::open(&address)?;

            drop(file);
            println!("Running server, press Ctrl + C to stop.");
            serve(listener, install_dir).await?;

            std::process::ExitStatus::default()
        }
        bdgm::runtime::Runtime::Windows => {
            if cfg!(target_os = "windows") {
                Command::new(install_dir.join(executable_str))
                    .args(game.args())
                    .envs(envvars)
                    .current_dir(&install_dir)
                    .status()?
            } else {
                Command::new(runtime_path.as_deref().unwrap_or("wine"))
                    .args(game.runtime_args())
                    .arg(install_dir.join(executable_str))
                    .args(game.args())
                    .envs(envvars)
                    .env("WINEPREFIX", game_dir.join("wineprefix"))
                    .current_dir(&install_dir)
                    .status()?
            }
        }
    };
    if !status.success() {
        eprintln!("Your game crashed: {status}");
        eprintln!("Setting a runtime with `--runtime /path/to/runtime` may fix your issue.");
    }

    Ok(())
}

const ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'/')
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');
