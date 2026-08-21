use anyhow::Error;
use serde::{Deserialize, Serialize};
use std::fmt::Write;

use crate::{error::ParserError, runtime::Runtime};

#[derive(Debug, Serialize, Deserialize)]
pub struct Game {
    pub name: String,
    pub id: String,
    pub version: String,
    pub executable: String,
    pub args: Vec<String>,
    pub runtime: Runtime,
    pub runtime_version: String,
    pub runtime_args: Vec<String>,
}

impl Game {
    fn new() -> Game {
        Game {
            name: String::new(),
            id: String::new(),
            version: String::new(),
            executable: String::new(),
            args: Vec::new(),
            runtime: Runtime::Java,
            runtime_version: String::new(),
            runtime_args: Vec::new(),
        }
    }

    pub fn to_string(&self) -> anyhow::Result<String> {
        let mut result = String::with_capacity(1024);
        result.push_str("BDGM/1.0\n");
        writeln!(result, "name={}", &self.name)?;
        writeln!(result, "id={}", &self.id)?;
        writeln!(result, "version={}", &self.version)?;
        writeln!(result, "executable={}", &self.executable)?;
        writeln!(
            result,
            "args={}",
            serde_json::ser::to_string(&self.runtime_args)?
        )?;
        writeln!(result, "runtime={}", &self.runtime)?;
        writeln!(result, "runtime_version={}", &self.runtime_version)?;
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
        if let Some(header) = lines.next()
            && header != "BDGM/1.0"
        {
            return Err(Error::from(ParserError::InvalidHeader));
        }

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
                "name" => result.name = value.to_string(),
                "id" => result.id = value.to_string(),
                "version" => result.version = value.to_string(),
                "executable" => result.executable = value.to_string(),
                "args" => result.args = serde_json::from_str(value)?,
                "runtime" => {
                    result.runtime = Runtime::from_str(value).ok_or(ParserError::InvalidRuntime)?
                }
                "runtime_version" => result.runtime_version = value.to_string(),
                "runtime_args" => result.runtime_args = serde_json::from_str(value)?,
                _ => {}
            }
        }

        if result.name == "" {
            Err(ParserError::MissingField("name".to_string()).into())
        } else if result.id == "" {
            Err(ParserError::MissingField("id".to_string()).into())
        } else if result.version == "" {
            Err(ParserError::MissingField("version".to_string()).into())
        } else if result.runtime_version == "" && !matches!(result.runtime, Runtime::Windows) {
            Err(ParserError::MissingField("runtime_version".to_string()).into())
        } else if result.executable == "" {
            Err(ParserError::MissingField("executable".to_string()).into())
        } else {
            Ok(result)
        }
    }
}

#[cfg(test)]
mod test {
    use std::io::Write;
    use std::{fs::File, io::Read};

    use crate::{game::Game, runtime::Runtime};

    #[test]
    fn game_serializes() {
        let game = Game {
            name: "test".to_string(),
            id: "com.example.test".to_string(),
            version: "1.0.0".to_string(),
            executable: "game.exe".to_string(),
            args: Vec::new(),
            runtime: Runtime::Windows,
            runtime_version: "10".to_string(),
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
        assert_eq!(game.name, "test");
        assert_eq!(game.id, "com.example.test");
        assert_eq!(game.version, "1.0.0");
        assert_eq!(game.executable, "game.exe");
        assert!(game.args.is_empty());
        assert_eq!(game.runtime, Runtime::Windows);
        assert_eq!(game.runtime_version, "10");
        assert!(game.runtime_args.is_empty());
    }
}
