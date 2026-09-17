use std::fmt::Display;

use serde::{Deserialize, Serialize};

/// An enum of runtimes supported by the library. This must match all runtimes of the latest
/// supported version of the spec. (see `ValidatedGame.get_supported_bdgm_versions`)
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum Runtime {
    #[serde(rename = "java")]
    Java,
    #[serde(rename = "dotnet")]
    Dotnet,
    #[serde(rename = "python")]
    Python,
    #[serde(rename = "windows")]
    Windows,
    #[serde(rename = "html")]
    HTML,
}

impl Display for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Runtime::Java => write!(f, "java"),
            Runtime::Dotnet => write!(f, "dotnet"),
            Runtime::Python => write!(f, "python"),
            Runtime::Windows => write!(f, "windows"),
            Runtime::HTML => write!(f, "html"),
        }
    }
}

impl Runtime {
    /// Creates a `Runtime` from the value of the `runtime` field of `DISC.BDGM`.
    pub fn from_str(str: &str) -> Option<Self> {
        match str {
            "java" => Some(Self::Java),
            "dotnet" => Some(Self::Dotnet),
            "python" => Some(Self::Python),
            "windows" => Some(Self::Windows),
            "html" => Some(Self::HTML),
            _ => None,
        }
    }

    /// Formats all runtimes as a comma-separated `String` of `DISC.BDGM` `runtime` field values.
    pub fn display_all() -> String {
        format!(
            "{}, {}, {}, {}, {}",
            Self::Java,
            Self::Dotnet,
            Self::Python,
            Self::Windows,
            Self::HTML,
        )
    }
}
