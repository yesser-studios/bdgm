#![allow(dead_code)]

use std::str::FromStr;

use bdgm::{
    error::BDGMErrors,
    game::{Game, ValidatedGame},
};

pub struct ManifestBuilder(Vec<String>);

impl ManifestBuilder {
    pub fn empty() -> Self {
        ManifestBuilder(Vec::new())
    }

    pub fn valid() -> Self {
        let fields = vec![
            "name=Example Game".to_string(),
            "id=com.example.example-game1".to_string(),
            "version=1.0.0".to_string(),
            "runtime=dotnet".to_string(),
            "runtime_version=10".to_string(),
            "executable=bin/ExampleGame.dll".to_string(),
            "args=[]".to_string(),
            "runtime_args=[]".to_string(),
        ];
        ManifestBuilder(fields)
    }

    pub fn build(&self) -> String {
        "BDGM/1.1\n".to_string() + &self.build_raw()
    }

    pub fn build_raw(&self) -> String {
        self.0.join("\n")
    }

    pub fn with(&mut self, key: &str, value: &str) -> &mut Self {
        self.0.push(format!("{key} = {value}"));
        self
    }

    pub fn without(&mut self, key: &str) -> &mut Self {
        self.0.retain(|l| !l.starts_with(key));
        self
    }

    pub fn with_raw(&mut self, line: impl Into<String>) -> &mut Self {
        self.0.push(line.into());
        self
    }

    pub fn valid_windows_without_runtime_version() -> Self {
        let mut builder = Self::valid();
        builder
            .without("runtime_version")
            .with("runtime", "windows");
        builder
    }

    pub fn valid_html_without_runtime_version() -> Self {
        let mut builder = Self::valid();
        builder.without("runtime_version").with("runtime", "html");
        builder
    }
}

pub fn validate_manifest(manifest: String) -> Result<ValidatedGame, BDGMErrors> {
    let game = Game::from_str(&manifest).map_err(BDGMErrors::from)?;
    ValidatedGame::validate(game)
}

mod tests {
    use super::*;

    #[test]
    fn valid_manifest_builder_is_valid() {
        let manifest = ManifestBuilder::valid().build();
        dbg!(&manifest);
        validate_manifest(manifest).expect("Game could not be validated.");
    }
    #[ignore = "Known bug: Windows runtime version is handled as mandatory."]
    #[test]
    fn valid_windows_manifest_builder_is_valid() {
        let manifest = ManifestBuilder::valid_windows_without_runtime_version().build();
        dbg!(&manifest);
        validate_manifest(manifest).expect("Game could not be validated.");
    }
    #[ignore = "Known bug: HTML runtime version is not ignored."]
    #[test]
    fn valid_html_manifest_builder_is_valid() {
        let manifest = ManifestBuilder::valid_html_without_runtime_version().build();
        dbg!(&manifest);
        validate_manifest(manifest).expect("Game could not be validated.");
    }
}
