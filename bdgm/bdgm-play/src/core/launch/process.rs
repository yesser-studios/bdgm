use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::ExitStatus,
    sync::Arc,
    time::Duration,
};

use bdgm::{game::ValidatedGame, runtime::Runtime};
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
    game: &ValidatedGame,
) -> Result<LaunchHandle> {
    let program = PathBuf::from(command.as_std().get_program());

    match command.kill_on_drop(true).spawn() {
        Ok(child) => Ok(LaunchHandle::GameProcess(GameProcess(Arc::new(
            AsyncMutex::new(child),
        )))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if cfg!(windows) && matches!(game.runtime(), Runtime::Windows) {
                Err(LaunchError::IoError(error).into())
            } else {
                Err(LaunchError::RuntimeNotFound(game.clone(), program).into())
            }
        }
        Err(error) => Err(LaunchError::IoError(error).into()),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::{OsStr, OsString},
        path::Path,
    };

    use bdgm::manifest_builder::ManifestBuilder;
    use tempfile::tempdir;

    use super::*;
    use crate::core::test_utils::{TestEnv, manifest, validated_game};

    fn installed(env: &TestEnv, manifest: &str) -> InstalledGame {
        env.installed(validated_game(manifest))
    }

    fn game_with(runtime: &str, executable: &str) -> ValidatedGame {
        let manifest = ManifestBuilder::valid()
            .with("runtime", runtime)
            .with("executable", executable)
            .build();

        ValidatedGame::validate_manifest(&manifest).expect("test manifest should be valid")
    }

    fn command_args(command: &tokio::process::Command) -> Vec<OsString> {
        command
            .as_std()
            .get_args()
            .map(|argument| argument.to_os_string())
            .collect()
    }

    #[test]
    fn envvars_include_spec_paths_and_a_wine_prefix_for_windows_games() {
        let env = TestEnv::new();
        let installed = installed(
            &env,
            &manifest(
                "com.example.test",
                "1.0.0",
                "windows",
                "game.exe",
                "[]",
                "[]",
            ),
        );

        let envvars = get_envvars(&installed);
        let data = installed.game_dirs.data();
        let cache = installed.game_dirs.cache();
        let app = installed.game_dirs.install_dir();
        let wineprefix = installed.game_dirs.root().join("wineprefix");

        assert_eq!(
            envvars.get("BDGM_DATA").map(String::as_str),
            Some(data.to_string_lossy().as_ref())
        );
        assert_eq!(
            envvars.get("BDGM_CACHE").map(String::as_str),
            Some(cache.to_string_lossy().as_ref())
        );
        assert_eq!(
            envvars.get("BDGM_APP").map(String::as_str),
            Some(app.to_string_lossy().as_ref())
        );
        assert_eq!(envvars.get("BDGM_VERSION").map(String::as_str), Some("1.1"));
        assert_eq!(
            envvars.get("WINEPREFIX").map(String::as_str),
            Some(wineprefix.to_string_lossy().as_ref())
        );
        assert!(data.is_absolute());
        assert!(cache.is_absolute());
        assert!(app.is_absolute());
    }

    #[test]
    fn non_windows_games_do_not_get_a_wine_prefix() {
        let env = TestEnv::new();
        let installed = installed(
            &env,
            &manifest("com.example.test", "1.0.0", "python", "game.py", "[]", "[]"),
        );

        let envvars = get_envvars(&installed);

        assert!(!envvars.contains_key("WINEPREFIX"));
    }

    #[test]
    fn java_places_the_jar_after_runtime_arguments() {
        let env = TestEnv::new();
        let installed = installed(
            &env,
            &manifest(
                "com.example.test",
                "1.0.0",
                "java",
                "game.jar",
                r#"["--foo", "bar"]"#,
                r#"["-Xmx1g"]"#,
            ),
        );

        let command = build_command(&installed, None).expect("build Java command");
        let executable = installed.game_dirs.executable();
        let install_dir = installed.game_dirs.install_dir();

        assert_eq!(command.as_std().get_program(), OsStr::new("java"));
        assert_eq!(
            command_args(&command),
            vec![
                OsString::from("-Xmx1g"),
                OsString::from("-jar"),
                executable.into_os_string(),
                OsString::from("--foo"),
                OsString::from("bar"),
            ]
        );
        assert_eq!(
            command.as_std().get_current_dir(),
            Some(install_dir.as_path())
        );
    }

    #[test]
    fn dotnet_and_python_pass_runtime_arguments_before_the_executable() {
        let env = TestEnv::new();

        for (runtime, executable) in [("dotnet", "game.dll"), ("python", "game.py")] {
            let installed = installed(
                &env,
                &manifest(
                    "com.example.test",
                    "1.0.0",
                    runtime,
                    executable,
                    r#"["--foo"]"#,
                    r#"["-u"]"#,
                ),
            );

            let command = build_command(&installed, None).expect("build command");
            let expected_executable = installed.game_dirs.executable();

            assert_eq!(command.as_std().get_program(), OsStr::new(runtime));
            assert_eq!(
                command_args(&command),
                vec![
                    OsString::from("-u"),
                    expected_executable.into_os_string(),
                    OsString::from("--foo"),
                ]
            );
        }
    }

    #[test]
    fn a_custom_runtime_replaces_the_default_binary() {
        let env = TestEnv::new();
        let installed = installed(
            &env,
            &manifest("com.example.test", "1.0.0", "python", "game.py", "[]", "[]"),
        );

        let command =
            build_command(&installed, Some(Path::new("/custom/runtime"))).expect("build command");

        assert_eq!(
            command.as_std().get_program(),
            OsStr::new("/custom/runtime")
        );
    }

    #[test]
    fn the_working_directory_is_the_executable_parent() {
        let env = TestEnv::new();
        let installed = installed(
            &env,
            &manifest(
                "com.example.test",
                "1.0.0",
                "python",
                "sub/game.py",
                "[]",
                "[]",
            ),
        );

        let command = build_command(&installed, None).expect("build command");
        let expected_dir = installed.game_dirs.install_dir().join("sub");

        assert_eq!(
            command.as_std().get_current_dir(),
            Some(expected_dir.as_path())
        );
    }

    #[test]
    fn html_games_cannot_be_built_as_processes() {
        let env = TestEnv::new();
        let installed = installed(
            &env,
            &manifest(
                "com.example.test",
                "1.0.0",
                "html",
                "index.html",
                "[]",
                "[]",
            ),
        );

        if let Err(AppError::Other(message)) = build_command(&installed, None) {
            assert!(
                message.contains("HTML games must be started through the HTML server"),
                "unexpected error message: {message}"
            );
        } else {
            panic!("HTML games must not produce a process command");
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn windows_games_use_wine_outside_windows() {
        let env = TestEnv::new();
        let installed = installed(
            &env,
            &manifest(
                "com.example.test",
                "1.0.0",
                "windows",
                "game.exe",
                r#"["--foo"]"#,
                r#"["-u"]"#,
            ),
        );

        let command = build_command(&installed, None).expect("build Wine command");
        let executable = installed.game_dirs.executable();

        assert_eq!(command.as_std().get_program(), OsStr::new("wine"));
        assert_eq!(
            command_args(&command),
            vec![
                OsString::from("-u"),
                executable.into_os_string(),
                OsString::from("--foo"),
            ]
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_games_run_natively_on_windows() {
        let env = TestEnv::new();
        let installed = installed(
            &env,
            &manifest(
                "com.example.test",
                "1.0.0",
                "windows",
                "game.exe",
                r#"["--foo"]"#,
                r#"["-u"]"#,
            ),
        );

        let command = build_command(&installed, None).expect("build native command");
        let executable = installed.game_dirs.executable();
        let install_dir = installed.game_dirs.install_dir();

        assert_eq!(command.as_std().get_program(), executable.as_os_str());
        assert_eq!(command_args(&command), vec![OsString::from("--foo")]);
        assert_eq!(
            command.as_std().get_current_dir(),
            Some(install_dir.as_path())
        );
    }

    #[tokio::test]
    async fn run_command_reports_a_missing_runtime() {
        let temp = tempdir().expect("create temporary directory");
        let missing = temp.path().join("missing-runtime");
        let command = tokio::process::Command::new(&missing);

        let result = run_command(command, &game_with("java", "game.jar"));

        assert!(matches!(
            result,
            Err(AppError::LaunchError(LaunchError::RuntimeNotFound(_, path))) if path == missing
        ));
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn run_command_reports_a_missing_native_windows_executable_as_io() {
        let temp = tempdir().expect("create temporary directory");
        let missing = temp.path().join("missing.exe");
        let command = tokio::process::Command::new(&missing);

        let result = run_command(command, &game_with("windows", "game.exe"));

        assert!(matches!(
            result,
            Err(AppError::LaunchError(LaunchError::IoError(_)))
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn run_command_returns_a_waitable_process() {
        let mut command = tokio::process::Command::new("sh");
        command.args(["-c", "exit 7"]);

        let handle = run_command(command, &game_with("python", "game.py")).expect("spawn shell");
        let LaunchHandle::GameProcess(process) = handle else {
            panic!("expected a game process");
        };

        let status = tokio::time::timeout(Duration::from_secs(5), process.wait())
            .await
            .expect("wait timed out")
            .expect("wait failed")
            .expect("process should have an exit status");

        assert_eq!(status.code(), Some(7));
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn run_command_returns_a_waitable_process_on_windows() {
        let mut command = tokio::process::Command::new("cmd");
        command.args(["/C", "exit", "7"]);

        let handle = run_command(command, &game_with("python", "game.py")).expect("spawn cmd");
        let LaunchHandle::GameProcess(process) = handle else {
            panic!("expected a game process");
        };

        let status = tokio::time::timeout(Duration::from_secs(5), process.wait())
            .await
            .expect("wait timed out")
            .expect("wait failed")
            .expect("process should have an exit status");

        assert_eq!(status.code(), Some(7));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn game_process_kill_terminates_the_process() {
        let mut command = tokio::process::Command::new("sh");
        command.args(["-c", "sleep 60"]);

        let handle = run_command(command, &game_with("python", "game.py")).expect("spawn shell");
        let LaunchHandle::GameProcess(process) = handle else {
            panic!("expected a game process");
        };

        process.kill().await;
        let status = tokio::time::timeout(Duration::from_secs(5), process.wait())
            .await
            .expect("wait timed out")
            .expect("wait failed")
            .expect("process should have an exit status");

        assert!(!status.success());
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn game_process_kill_terminates_the_process_on_windows() {
        let mut command = tokio::process::Command::new("ping");
        command.args(["-n", "60", "127.0.0.1"]);

        let handle = run_command(command, &game_with("python", "game.py")).expect("spawn ping");
        let LaunchHandle::GameProcess(process) = handle else {
            panic!("expected a game process");
        };

        process.kill().await;
        let status = tokio::time::timeout(Duration::from_secs(5), process.wait())
            .await
            .expect("wait timed out")
            .expect("wait failed")
            .expect("process should have an exit status");

        assert!(!status.success());
    }
}
