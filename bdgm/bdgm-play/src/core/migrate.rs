use std::{ffi::OsStr, fs, io, path::Path};

use platform_dirs::AppDirs;

const GAMES_DIR: &str = "games";

/// Moves game directories written by older versions into the `games/` subtree.
///
/// Older versions stored each game directly under the app data and cache
/// directories (`<data_dir>/<id>` and `<cache_dir>/<id>`). Current versions
/// nest them under `games/`. This is best-effort: every entry is attempted,
/// and the first error is returned for the caller to report.
pub(super) fn migrate_dirs(app_dirs: &AppDirs) -> io::Result<()> {
    let mut first_error = None;

    for parent in [&app_dirs.data_dir, &app_dirs.cache_dir] {
        if let Err(error) = migrate_parent(parent)
            && first_error.is_none()
        {
            first_error = Some(error);
        }
    }

    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// Attempts to migrate each legacy child directory, ignoring a missing parent.
///
/// Returns the first directory read or migration error after attempting all entries.
fn migrate_parent(parent: &Path) -> io::Result<()> {
    let entries = match fs::read_dir(parent) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };

    let games = parent.join(GAMES_DIR);
    let mut first_error = None;

    for entry in entries {
        if let Err(error) = migrate_entry(&games, entry)
            && first_error.is_none()
        {
            first_error = Some(error);
        }
    }

    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// Moves a directory into `games`, skipping files, `games` itself, and existing destinations.
///
/// Propagates entry inspection, directory creation, and rename errors.
fn migrate_entry(games: &Path, entry: io::Result<fs::DirEntry>) -> io::Result<()> {
    let entry = entry?;
    let file_type = entry.file_type()?;

    if !file_type.is_dir() || entry.file_name() == OsStr::new(GAMES_DIR) {
        return Ok(());
    }

    let destination = games.join(entry.file_name());
    if destination.exists() {
        return Ok(());
    }

    fs::create_dir_all(games)?;
    fs::rename(entry.path(), destination)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::core::test_utils::TestEnv;

    #[test]
    fn migrates_legacy_data_and_cache_dirs_into_games() {
        let env = TestEnv::new();
        let legacy_data = env.app_dirs.data_dir.join("com.example.test");
        let legacy_cache = env.app_dirs.cache_dir.join("com.example.test");

        let install = legacy_data.join("app").join("1.0.0");
        fs::create_dir_all(&install).expect("create legacy install dir");
        fs::write(install.join("game.exe"), b"exe").expect("write installed executable");

        let data = legacy_data.join("data");
        fs::create_dir_all(&data).expect("create legacy data dir");
        fs::write(data.join("save.dat"), b"save").expect("write save");

        let cache = legacy_cache.join("cache");
        fs::create_dir_all(&cache).expect("create legacy cache dir");
        fs::write(cache.join("asset"), b"asset").expect("write cached asset");

        migrate_dirs(&env.app_dirs).expect("migrate legacy dirs");

        let data_root = env.app_dirs.data_dir.join("games").join("com.example.test");
        let cache_root = env
            .app_dirs
            .cache_dir
            .join("games")
            .join("com.example.test");

        assert!(!legacy_data.exists());
        assert!(!legacy_cache.exists());
        assert_eq!(
            fs::read(data_root.join("app").join("1.0.0").join("game.exe"))
                .expect("read migrated executable"),
            b"exe"
        );
        assert_eq!(
            fs::read(data_root.join("data").join("save.dat")).expect("read migrated save"),
            b"save"
        );
        assert_eq!(
            fs::read(cache_root.join("cache").join("asset")).expect("read migrated asset"),
            b"asset"
        );
    }

    #[test]
    fn does_not_overwrite_an_existing_game_directory() {
        let env = TestEnv::new();
        let legacy = env.app_dirs.data_dir.join("com.example.test");
        let existing = env.app_dirs.data_dir.join("games").join("com.example.test");

        fs::create_dir_all(legacy.join("data")).expect("create legacy data dir");
        fs::write(legacy.join("data").join("old-save"), b"old").expect("write legacy save");
        fs::create_dir_all(&existing).expect("create existing game dir");
        fs::write(existing.join("new-save"), b"new").expect("write new save");

        migrate_dirs(&env.app_dirs).expect("migrate legacy dirs");

        assert!(legacy.join("data").join("old-save").is_file());
        assert_eq!(
            fs::read(existing.join("new-save")).expect("read new save"),
            b"new"
        );
        assert!(!existing.join("data").exists());
    }

    #[test]
    fn ignores_files_and_the_games_directory() {
        let env = TestEnv::new();
        let games = env.app_dirs.data_dir.join("games");

        fs::create_dir_all(&games).expect("create games dir");
        fs::write(env.app_dirs.data_dir.join("ports.json"), b"{}").expect("write ports file");

        migrate_dirs(&env.app_dirs).expect("migrate legacy dirs");

        assert!(env.app_dirs.data_dir.join("ports.json").is_file());
        assert!(games.is_dir());
        assert_eq!(fs::read_dir(&games).expect("read games dir").count(), 0);
    }

    #[test]
    fn missing_data_and_cache_dirs_are_ignored() {
        let env = TestEnv::new();

        migrate_dirs(&env.app_dirs).expect("migrate missing dirs");

        assert!(!env.app_dirs.data_dir.exists());
        assert!(!env.app_dirs.cache_dir.exists());
    }

    #[test]
    fn migration_is_idempotent() {
        let env = TestEnv::new();
        let legacy = env.app_dirs.data_dir.join("com.example.test");
        let migrated = env.app_dirs.data_dir.join("games").join("com.example.test");

        fs::create_dir_all(legacy.join("data")).expect("create legacy data dir");
        fs::write(legacy.join("data").join("save.dat"), b"save").expect("write save");

        migrate_dirs(&env.app_dirs).expect("migrate legacy dirs");
        migrate_dirs(&env.app_dirs).expect("migrate again");

        assert!(!legacy.exists());
        assert_eq!(
            fs::read(migrated.join("data").join("save.dat")).expect("read migrated save"),
            b"save"
        );
    }
}
