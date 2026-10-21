#![allow(dead_code)]

use bdgm::game::ValidatedGame;

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
        self.without(key);
        self.0.push(format!("{key}={value}"));
        self
    }

    pub fn without(&mut self, key: &str) -> &mut Self {
        self.0.retain(|line| {
            line.split_once('=')
                .is_none_or(|(line_key, _)| line_key != key)
        });
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

mod tests {
    use super::*;

    #[test]
    fn valid_manifest_builder_is_valid() {
        let manifest = ManifestBuilder::valid().build();
        ValidatedGame::validate_manifest(&manifest).expect("Game could not be validated.");
    }
    #[test]
    fn valid_windows_manifest_builder_is_valid() {
        let manifest = ManifestBuilder::valid_windows_without_runtime_version().build();
        ValidatedGame::validate_manifest(&manifest).expect("Game could not be validated.");
    }
    #[test]
    fn valid_html_manifest_builder_is_valid() {
        let manifest = ManifestBuilder::valid_html_without_runtime_version().build();
        ValidatedGame::validate_manifest(&manifest).expect("Game could not be validated.");
    }
    #[test]
    fn without_only_removes_the_exact_key() {
        let mut builder = ManifestBuilder::valid();
        builder.without("runtime");

        let manifest = builder.build_raw();
        assert!(!manifest.contains("runtime="));
        assert!(manifest.contains("runtime_version=10"));
        assert!(manifest.contains("runtime_args=[]"));
    }
}
