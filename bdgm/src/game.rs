use anyhow::Error;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Write,
    path::{Path, PathBuf},
};

use crate::{
    error::{
        BDGMError::{self},
        BDGMErrors, ExecutableError, IdFormatError, ParserError,
    },
    runtime::Runtime,
};

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

    pub fn to_string(&self) -> anyhow::Result<String> {
        let mut result = String::with_capacity(1024);
        if let Some(bdgm_version) = &self.bdgm_version {
            writeln!(result, "BDGM/{}", bdgm_version)?;
        }
        if let Some(name) = &self.name {
            writeln!(result, "name={}", name)?
        };
        if let Some(id) = &self.id {
            writeln!(result, "id={}", id)?;
        }
        if let Some(version) = &self.version {
            writeln!(result, "version={}", version)?;
        }
        if let Some(executable) = &self.executable {
            writeln!(result, "executable={}", executable)?;
        }
        writeln!(
            result,
            "args={}",
            serde_json::ser::to_string(&self.runtime_args)?
        )?;
        if let Some(runtime) = &self.runtime {
            writeln!(result, "runtime={}", runtime)?;
        }
        if let Some(runtime_version) = &self.runtime_version {
            writeln!(result, "runtime_version={}", runtime_version)?;
        }
        writeln!(
            result,
            "runtime_args={}",
            serde_json::ser::to_string(&self.runtime_args)?
        )?;
        Ok(result)
    }

    pub fn from_str(str: &str) -> anyhow::Result<Game> {
        let mut result = Game::new();

        let mut lines = str.lines();
        result.bdgm_version = match lines.next() {
            Some(header) => {
                let split: Vec<_> = header.split('/').collect();
                if split.len() != 2 || split.get(0).is_none_or(|x| *x != "BDGM") {
                    return Err(Error::from(ParserError::InvalidHeader));
                }
                match split.get(1) {
                    Some(x) => Some(x.to_string()),
                    None => None,
                }
            }
            None => None,
        };

        for line in lines {
            if let Some(char) = line.chars().nth(0)
                && char == '#'
            {
                continue;
            }
            if !line.contains("=") {
                continue;
            }

            let split = line.split("=").collect::<Vec<&str>>();
            if split.len() != 2 {
                continue;
            }
            let key = split[0];
            let value = split[1];
            match key {
                "name" => result.name = Some(value.to_string()),
                "id" => result.id = Some(value.to_string()),
                "version" => result.version = Some(value.to_string()),
                "executable" => result.executable = Some(value.to_string()),
                "args" => result.args = serde_json::from_str(value)?,
                "runtime" => result.runtime = Some(value.to_string()),
                "runtime_version" => result.runtime_version = Some(value.to_string()),
                "runtime_args" => result.runtime_args = serde_json::from_str(value)?,
                _ => {}
            }
        }

        Ok(result)
    }
}

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
    pub fn get_supported_bdgm_versions() -> Vec<String> {
        vec!["1.0".to_string(), "1.1".to_string()]
    }

    pub fn validate(game: Game) -> Result<Self, BDGMErrors> {
        let mut errors = BDGMErrors(Vec::new());

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
                let runtime = Runtime::from_str(&runtime_string);
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
                && id.split('.').count() >= 2
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

        if let Some(runtime) = &runtime {
            if *runtime != Runtime::Windows
                && *runtime != Runtime::HTML
                && runtime_version.is_none()
            {
                errors.add(BDGMError::MandatoryFieldMissing(
                    "runtime_version".to_string(),
                ));
            }
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
    use std::io::Write;
    use std::{fs::File, io::Read};

    use crate::game::ValidatedGame;
    use crate::{game::Game, runtime::Runtime};

    #[test]
    fn game_serializes() {
        let game = Game {
            bdgm_version: Some("1.1".to_string()),
            name: Some("test".to_string()),
            id: Some("com.example.test".to_string()),
            version: Some("1.0.0".to_string()),
            executable: Some("game.exe".to_string()),
            args: Vec::new(),
            runtime: Some(Runtime::Windows.to_string()),
            runtime_version: Some("10".to_string()),
            runtime_args: Vec::new(),
        };

        let result = game.to_string().unwrap();
        let mut file = File::create("./DISC_write.BDGM").unwrap();
        write!(file, "{result}").unwrap();
    }

    #[test]
    fn game_deserializes() {
        let mut file = File::open("./DISC_read.BDGM").unwrap();
        let mut result = String::new();
        file.read_to_string(&mut result).unwrap();
        let game = Game::from_str(&result).unwrap();
        assert_eq!(game.name.unwrap(), "test");
        assert_eq!(game.id.unwrap(), "com.example.test");
        assert_eq!(game.version.unwrap(), "1.0.0");
        assert_eq!(game.executable.unwrap(), "game.exe");
        assert!(game.args.is_empty());
        assert_eq!(game.runtime.unwrap(), Runtime::Windows.to_string());
        assert_eq!(game.runtime_version.unwrap(), "10");
        assert!(game.runtime_args.is_empty());
    }

    #[test]
    fn game_validates() {
        let game = Game {
            bdgm_version: Some("1.1".to_string()),
            name: Some("test".to_string()),
            id: Some("com.example.test".to_string()),
            version: Some("1.0.0".to_string()),
            executable: Some("game.exe".to_string()),
            args: Vec::new(),
            runtime: Some(Runtime::Windows.to_string()),
            runtime_version: Some("10".to_string()),
            runtime_args: Vec::new(),
        };

        let validated = match ValidatedGame::validate(game.clone()) {
            Ok(game) => game,
            Err(e) => panic!("Validation failed: {e}"),
        };

        assert_eq!(game, validated.to_game());
    }
}
