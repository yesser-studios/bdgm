use crate::{
    error::{
        BDGMError::{self, ArgsInvalid, RuntimeArgsInvalid},
        BDGMErrors, ExecutableError, IdFormatError, ParserError, VersionFormatError,
    },
    runtime::Runtime,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fmt::Write,
    path::{Path, PathBuf},
    str::FromStr,
};

/// Raw game struct. Used for parsing DISC.BDGM files using `from_str`.
/// Use `ValidatedGame::validate` to validate and use `ValidatedGame`-provided getters.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Game {
    pub bdgm_version: Option<String>,
    pub name: Option<String>,
    pub id: Option<String>,
    pub version: Option<String>,
    pub executable: Option<String>,
    pub args: Vec<String>,
    pub runtime: Option<String>,
    pub runtime_version: Option<String>,
    pub runtime_args: Vec<String>,
}

impl Game {
    /// Creates a new empty Game. This is not valid by default as fields need to be populated.
    fn new() -> Game {
        Game {
            bdgm_version: None,
            name: None,
            id: None,
            version: None,
            executable: None,
            args: Vec::new(),
            runtime: None,
            runtime_version: None,
            runtime_args: Vec::new(),
        }
    }
}

impl FromStr for Game {
    type Err = BDGMErrors;

    /// Parses a `Game` from `DISC.BDGM` file contents.
    /// Comments and lines without `=` after the header are ignored. Unknown keys
    /// are ignored after checking their syntax. Values retain any embedded `=`.
    ///
    /// Returns collected header, whitespace, empty-value, duplicate-key, and JSON
    /// argument errors. Required fields and field values are checked separately
    /// by `ValidatedGame::validate`; an empty input produces an empty `Game`.
    fn from_str(s: &str) -> Result<Game, Self::Err> {
        let mut errors = BDGMErrors::default();
        let mut result = Game::new();

        let mut lines = s.lines();
        result.bdgm_version = match lines.next() {
            Some(header) => {
                let split: Vec<_> = header.split('/').collect();
                if split.len() != 2 || split.first().is_none_or(|x| *x != "BDGM") {
                    errors.add(ParserError::InvalidHeader.into());
                }
                split.get(1).map(|x| x.to_string())
            }
            None => None,
        };

        let mut used = HashSet::new();
        for line in lines {
            if let Some(char) = line.chars().nth(0)
                && char == '#'
            {
                continue;
            }
            if !line.contains("=") {
                continue;
            }

            let (key, value) = match line.split_once("=") {
                Some(s) => s,
                None => continue,
            };

            if line.chars().nth(0).is_some_and(|c| c.is_whitespace()) {
                errors.add(ParserError::LeadingWhitespace(line.to_string()).into());
            } else if key.chars().any(|c| c.is_whitespace()) {
                errors.add(ParserError::WhitespaceInKey(key.to_string()).into());
            }
            if line.chars().nth_back(0).is_some_and(|c| c.is_whitespace()) {
                errors.add(ParserError::TrailingWhitespace(line.to_string()).into());
            }

            if value.is_empty() {
                errors.add(ParserError::EmptyValue(key.to_string()).into());
            }

            if used.contains(key) {
                errors.add(ParserError::DuplicateKey(key.to_string()).into());
            }

            used.insert(key.to_string());

            match key {
                "name" => result.name = Some(value.to_string()),
                "id" => result.id = Some(value.to_string()),
                "version" => result.version = Some(value.to_string()),
                "executable" => result.executable = Some(value.to_string()),
                "args" => match serde_json::from_str(value) {
                    Ok(v) => result.args = v,
                    Err(e) => errors.add(ArgsInvalid(e.to_string())),
                },
                "runtime" => result.runtime = Some(value.to_string()),
                "runtime_version" => result.runtime_version = Some(value.to_string()),
                "runtime_args" => match serde_json::from_str(value) {
                    Ok(v) => result.runtime_args = v,
                    Err(e) => errors.add(RuntimeArgsInvalid(e.to_string())),
                },
                _ => {}
            }
        }

        errors.evaluate()?;
        Ok(result)
    }
}

/// `ValidatedGame` provides compile-time guarantees that the game is valid. Use the `validate`
/// constructor to validate a raw `Game` and get a `ValidatedGame`.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct ValidatedGame {
    bdgm_version: String,
    name: String,
    id: String,
    version: String,
    executable: PathBuf,
    args: Vec<String>,
    runtime: Runtime,
    runtime_version: Option<String>,
    runtime_args: Vec<String>,
}

trait IsNoneOrEmpty {
    fn is_none_or_empty(&self) -> bool;
}
impl IsNoneOrEmpty for Option<String> {
    fn is_none_or_empty(&self) -> bool {
        self.as_ref().is_none_or(|val| val.is_empty())
    }
}

impl ValidatedGame {
    /// Gets a `Vec` of supported BDGM spec versions by this library.
    pub fn get_supported_bdgm_versions() -> Vec<String> {
        vec!["1.0".to_string(), "1.1".to_string()]
    }

    /// Validates a raw `Game` and returns a `ValidatedGame` or `BDGMErrors`, a set of validation
    /// errors.
    ///
    /// Checks required fields, supported BDGM versions, ID and version restrictions,
    /// runtime names, and executable path syntax. A runtime version is required for
    /// Java, .NET, and Python, but its value is not checked. This does not inspect the
    /// filesystem or check whether a compatible runtime is installed.
    pub fn validate(game: Game) -> Result<Self, BDGMErrors> {
        let mut errors = BDGMErrors::default();

        let bdgm_version = if game.bdgm_version.is_none_or_empty() {
            errors.add(BDGMError::MissingHeaderVersion);
            "".to_string()
        } else {
            let bdgm_version = game.bdgm_version.unwrap();
            if !Self::get_supported_bdgm_versions().contains(&bdgm_version) {
                errors.add(BDGMError::UnsupportedHeaderVersion(bdgm_version));
                "".to_string()
            } else {
                bdgm_version
            }
        };
        let name = if game.name.is_none_or_empty() {
            errors.add(BDGMError::MandatoryFieldMissing("name".to_string()));
            "".to_string()
        } else {
            game.name.unwrap()
        };
        let id = if game.id.is_none_or_empty() {
            errors.add(BDGMError::MandatoryFieldMissing("id".to_string()));
            "".to_string()
        } else {
            game.id.unwrap()
        };
        let version = if game.version.is_none_or_empty() {
            errors.add(BDGMError::MandatoryFieldMissing("version".to_string()));
            "".to_string()
        } else {
            game.version.unwrap()
        };
        let executable = if game.executable.is_none_or_empty() {
            errors.add(BDGMError::MandatoryFieldMissing("executable".to_string()));
            "".to_string()
        } else {
            game.executable.unwrap()
        };
        let args = game.args;
        let runtime = match game.runtime {
            Some(runtime_string) => {
                let runtime = Runtime::from_name(&runtime_string);
                if runtime.is_none() {
                    errors.add(BDGMError::RuntimeInvalid(runtime_string));
                }
                runtime
            }
            None => {
                errors.add(BDGMError::MandatoryFieldMissing("runtime".to_string()));
                None
            }
        };
        let runtime_version = game.runtime_version;
        let runtime_args = game.runtime_args;

        if !id.is_empty() {
            if id.chars().any(|c| c.is_whitespace()) {
                errors.add(BDGMError::IdFormatInvalid(
                    IdFormatError::ContainsWhitespace,
                ));
            }
            if id.chars().any(|c| c.is_ascii_uppercase()) {
                errors.add(BDGMError::IdFormatInvalid(IdFormatError::ContainsUppercase));
            }
            if id.contains('_') {
                errors.add(BDGMError::IdFormatInvalid(
                    IdFormatError::ContainsUnderscores,
                ));
            }
            if let Some(c) = id.chars().find(|c| {
                !(c.is_ascii_lowercase()
                    || c.is_ascii_digit()
                    || *c == '-'
                    || *c == '.'
                    || c.is_ascii_uppercase() // Handled above
                    || *c == '_' // Handled above
                    || c.is_whitespace()) // Handled above
            }) {
                errors.add(BDGMError::IdFormatInvalid(
                    IdFormatError::ContainsForbiddenCharacter(c),
                ));
            }
            if !(id.contains('.')
                && id.split('.').count() >= 3
                && id.split('.').all(|seg| {
                    !seg.is_empty()
                        && !seg.starts_with('-')
                        && !seg.ends_with('-')
                        && !seg.contains("--")
                }))
            {
                errors.add(BDGMError::IdFormatInvalid(
                    IdFormatError::IncorrectStructure,
                ))
            }
        }

        if !(version.is_empty()) {
            for forbidden_char in ['/', '\\', ':', '*', '?', '"', '<', '>', '|'] {
                if version
                    .to_lowercase()
                    .contains(&forbidden_char.to_lowercase().to_string())
                {
                    errors
                        .add(VersionFormatError::ContainsForbiddenCharacter(forbidden_char).into());
                }
            }
            for forbidden_ver in ["..", "."] {
                if version.to_lowercase() == forbidden_ver.to_lowercase() {
                    errors.add(
                        VersionFormatError::ForbiddenVersion(forbidden_ver.to_string()).into(),
                    );
                }
            }
            for forbidden_seq in ["CON", "PRN", "AUX", "NUL"]
                .into_iter()
                .map(str::to_string)
                .chain((0..=9).flat_map(|n| [format!("COM{n}"), format!("LPT{n}")]))
            {
                if version
                    .to_lowercase()
                    .contains(&forbidden_seq.to_lowercase())
                {
                    errors.add(
                        VersionFormatError::ContainsForbiddenSequence(forbidden_seq.to_string())
                            .into(),
                    );
                }
            }
            for forbidden_end in [".part", ".lock", ".", " "] {
                if version
                    .to_lowercase()
                    .ends_with(&forbidden_end.to_lowercase())
                {
                    errors.add(
                        VersionFormatError::EndsWithForbiddenSequence(forbidden_end.to_string())
                            .into(),
                    );
                }
            }
        }

        if let Some(runtime) = &runtime
            && *runtime != Runtime::Windows
            && *runtime != Runtime::HTML
            && runtime_version.is_none()
        {
            errors.add(BDGMError::MandatoryFieldMissing(
                "runtime_version".to_string(),
            ));
        }

        let executable: Option<PathBuf> = if !executable.is_empty() {
            if executable.contains('\\') {
                errors.add(BDGMError::ExecutablePathForbidden(
                    ExecutableError::BackslashSeparated,
                ));
            }
            let executable_path = Path::new(&executable);
            if executable_path.is_absolute() {
                errors.add(BDGMError::ExecutablePathForbidden(
                    ExecutableError::IsAbsolute,
                ));
            }
            for component in executable_path.components() {
                match component {
                    std::path::Component::Normal(_) => {}
                    std::path::Component::CurDir => errors.add(BDGMError::ExecutablePathForbidden(
                        ExecutableError::ContainsCurrentDir,
                    )),
                    std::path::Component::ParentDir => errors.add(
                        BDGMError::ExecutablePathForbidden(ExecutableError::ContainsParentDir),
                    ),
                    std::path::Component::Prefix(_) | std::path::Component::RootDir => errors.add(
                        BDGMError::ExecutablePathForbidden(ExecutableError::IsAbsolute),
                    ),
                }
            }
            Some(executable_path.to_path_buf())
        } else {
            None
        };

        errors.evaluate()?;

        Ok(ValidatedGame {
            bdgm_version,
            name,
            id,
            version,
            executable: executable.expect("Executable is None but should have errored above"),
            args,
            runtime: runtime.expect("Runtime is None but should have errored above"),
            runtime_version,
            runtime_args,
        })
    }

    /// Converts `ValidatedGame` into a valid `DISC.BDGM` string.
    pub fn to_string(&self) -> anyhow::Result<String> {
        let mut result = String::with_capacity(1024);
        writeln!(result, "BDGM/{}", self.bdgm_version)?;
        writeln!(result, "name={}", self.name)?;
        writeln!(result, "id={}", self.id)?;
        writeln!(result, "version={}", self.version)?;
        writeln!(result, "executable={}", self.executable.to_string_lossy())?;
        writeln!(result, "runtime={}", self.runtime)?;

        if let Some(runtime_version) = &self.runtime_version {
            writeln!(result, "runtime_version={}", runtime_version)?;
        }

        writeln!(result, "args={}", serde_json::ser::to_string(&self.args)?)?;
        writeln!(
            result,
            "runtime_args={}",
            serde_json::ser::to_string(&self.runtime_args)?
        )?;
        Ok(result)
    }

    /// Parses and validates `DISC.BDGM` contents without accessing the filesystem.
    ///
    /// Returns parsing errors first; field validation runs only if parsing succeeds.
    pub fn validate_manifest(manifest: &str) -> Result<Self, BDGMErrors> {
        let game = Game::from_str(manifest)?;
        Self::validate(game)
    }

    pub fn bdgm_version(&self) -> &str {
        &self.bdgm_version
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn executable(&self) -> &PathBuf {
        &self.executable
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }

    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    pub fn runtime_version(&self) -> Option<&str> {
        match &self.runtime_version {
            Some(version) => Some(version),
            None => None,
        }
    }

    pub fn runtime_args(&self) -> &[String] {
        &self.runtime_args
    }

    pub fn to_game(self) -> Game {
        Game::from(self)
    }
}

impl From<ValidatedGame> for Game {
    fn from(value: ValidatedGame) -> Self {
        Game {
            bdgm_version: Some(value.bdgm_version),
            name: Some(value.name),
            id: Some(value.id),
            version: Some(value.version),
            executable: Some(value.executable.to_string_lossy().into_owned()),
            args: value.args,
            runtime: Some(value.runtime.to_string()),
            runtime_version: value.runtime_version,
            runtime_args: value.runtime_args,
        }
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use crate::game::{Game, ValidatedGame};
    use crate::runtime::Runtime;

    fn sample_game() -> Game {
        Game {
            bdgm_version: Some("1.1".to_string()),
            name: Some("test".to_string()),
            id: Some("com.example.test".to_string()),
            version: Some("1.0.0".to_string()),
            executable: Some("game.exe".to_string()),
            args: Vec::new(),
            runtime: Some(Runtime::Windows.to_string()),
            runtime_version: Some("10".to_string()),
            runtime_args: Vec::new(),
        }
    }

    #[test]
    fn game_serializes() {
        let serialized = ValidatedGame::validate(sample_game())
            .expect("game should validate")
            .to_string()
            .expect("game should serialize");

        assert_eq!(
            serialized,
            "BDGM/1.1\n\
             name=test\n\
             id=com.example.test\n\
             version=1.0.0\n\
             executable=game.exe\n\
             runtime=windows\n\
             runtime_version=10\n\
             args=[]\n\
             runtime_args=[]\n"
        );
    }

    #[test]
    fn game_serialization_round_trips() {
        let game = sample_game();
        let serialized = ValidatedGame::validate(game.clone())
            .expect("game should validate")
            .to_string()
            .expect("game should serialize");

        assert_eq!(Game::from_str(&serialized).unwrap(), game);
    }

    #[test]
    fn game_deserializes() {
        let game = Game::from_str(include_str!("../DISC_read.BDGM")).unwrap();
        assert_eq!(game.bdgm_version.as_deref(), Some("1.1"));
        assert_eq!(game.name.as_deref(), Some("test"));
        assert_eq!(game.id.as_deref(), Some("com.example.test"));
        assert_eq!(game.version.as_deref(), Some("1.0.0"));
        assert_eq!(game.executable.as_deref(), Some("game.exe"));
        assert!(game.args.is_empty());
        assert_eq!(game.runtime.as_deref(), Some("windows"));
        assert_eq!(game.runtime_version.as_deref(), Some("10"));
        assert!(game.runtime_args.is_empty());
    }

    #[test]
    fn game_validates() {
        let game = sample_game();

        let validated = match ValidatedGame::validate(game.clone()) {
            Ok(game) => game,
            Err(e) => panic!("Validation failed: {e}"),
        };

        assert_eq!(game, validated.to_game());
    }
}
