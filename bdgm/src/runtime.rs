use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Runtime {
    #[serde(rename = "java")]
    Java,
    #[serde(rename = "dotnet")]
    Dotnet,
    #[serde(rename = "python")]
    Python,
    #[serde(rename = "windows")]
    Windows,
}

impl Display for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Runtime::Java => write!(f, "java"),
            Runtime::Dotnet => write!(f, "dotnet"),
            Runtime::Python => write!(f, "python"),
            Runtime::Windows => write!(f, "windows"),
        }
    }
}

impl Runtime {
    pub fn from_str(str: &str) -> Option<Self> {
        match str {
            "java" => Some(Self::Java),
            "dotnet" => Some(Self::Dotnet),
            "python" => Some(Self::Python),
            "windows" => Some(Self::Windows),
            _ => None,
        }
    }
}
