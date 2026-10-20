use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::ExitStatus,
    sync::Arc,
    time::Duration,
};

use bdgm::runtime::Runtime;
use tokio::sync::Mutex as AsyncMutex;

use crate::core::{
    error::{AppError, LaunchError, Result, RunningError},
    request::InstalledGame,
};

use super::handle::LaunchHandle;

#[derive(Clone)]
pub struct GameProcess(Arc<AsyncMutex<tokio::process::Child>>);

impl GameProcess {
    pub async fn kill(&self) {
        let mut child = self.0.lock().await;
        let _ = child.start_kill();
    }

    pub async fn wait(self) -> Result<Option<ExitStatus>> {
        loop {
            {
                let mut child = self.0.lock().await;

                match child.try_wait() {
                    Ok(Some(status)) => return Ok(Some(status)),
                    Ok(None) => {}
                    Err(error) => return Err(RunningError::IoError(error).into()),
                }
            }

            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }
}

pub(super) fn build_command(
    installed: &InstalledGame,
    runtime: Option<&Path>,
) -> Result<tokio::process::Command> {
    let envvars = get_envvars(installed);
    let game = &installed.game;
    let game_dirs = &installed.game_dirs;
    let executable = game_dirs.executable();
    let install_dir = game_dirs.install_dir();

    let runtime_args: Vec<&str> = game.runtime_args().iter().map(String::as_str).collect();
    let args: Vec<&str> = game.args().iter().map(String::as_str).collect();

    match game.runtime() {
        Runtime::Java => {
            let mut java_runtime_args = runtime_args.clone();
            java_runtime_args.push("-jar");

            build_bin_command(
                Path::new("java"),
                &executable,
                &install_dir,
                runtime,
                &java_runtime_args,
                &args,
                &envvars,
            )
        }
        Runtime::Dotnet => build_bin_command(
            Path::new("dotnet"),
            &executable,
            &install_dir,
            runtime,
            &runtime_args,
            &args,
            &envvars,
        ),
        Runtime::Python => build_bin_command(
            Path::new("python"),
            &executable,
            &install_dir,
            runtime,
            &runtime_args,
            &args,
            &envvars,
        ),
        Runtime::Windows => {
            if cfg!(windows) {
                let mut command = tokio::process::Command::new(&executable);
                command.args(&args);
                command.envs(&envvars);
                command.current_dir(executable.parent().unwrap_or(&install_dir));
                Ok(command)
            } else {
                build_bin_command(
                    Path::new("wine"),
                    &executable,
                    &install_dir,
                    runtime,
                    &runtime_args,
                    &args,
                    &envvars,
                )
            }
        }
        Runtime::HTML => Err(AppError::Other(
            "HTML games must be started through the HTML server".to_string(),
        )),
    }
}

fn get_envvars(installed: &InstalledGame) -> HashMap<&'static str, String> {
    let game_dirs = &installed.game_dirs;
    let mut envvars = HashMap::new();

    envvars.insert("BDGM_DATA", game_dirs.data().to_string_lossy().into_owned());
    envvars.insert(
        "BDGM_CACHE",
        game_dirs.cache().to_string_lossy().into_owned(),
    );
    envvars.insert(
        "BDGM_APP",
        game_dirs.install_dir().to_string_lossy().into_owned(),
    );
    envvars.insert("BDGM_VERSION", installed.game.bdgm_version().to_string());

    if matches!(installed.game.runtime(), Runtime::Windows) {
        envvars.insert(
            "WINEPREFIX",
            game_dirs
                .root()
                .join("wineprefix")
                .to_string_lossy()
                .into_owned(),
        );
    }

    envvars
}

fn build_bin_command(
    default_runtime: &Path,
    executable: &Path,
    install_dir: &Path,
    runtime: Option<&Path>,
    runtime_args: &[&str],
    args: &[&str],
    envvars: &HashMap<&'static str, String>,
) -> Result<tokio::process::Command> {
    let program = runtime.unwrap_or(default_runtime);
    let mut command = tokio::process::Command::new(program);

    command
        .args(runtime_args)
        .arg(executable)
        .args(args)
        .envs(envvars)
        .current_dir(executable.parent().unwrap_or(install_dir));

    Ok(command)
}

pub(super) fn run_command(
    mut command: tokio::process::Command,
    runtime: &Runtime,
) -> Result<LaunchHandle> {
    let program = PathBuf::from(command.as_std().get_program());

    match command.kill_on_drop(true).spawn() {
        Ok(child) => Ok(LaunchHandle::GameProcess(GameProcess(Arc::new(
            AsyncMutex::new(child),
        )))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if cfg!(windows) && matches!(runtime, Runtime::Windows) {
                Err(LaunchError::IoError(error).into())
            } else {
                Err(LaunchError::RuntimeNotFound(program).into())
            }
        }
        Err(error) => Err(LaunchError::IoError(error).into()),
    }
}
