pub mod language;
pub mod scenario;
pub mod test;
pub mod util;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};
use test::{from_base64, to_base64};
use thiserror::Error;

#[derive(Display, PartialEq, Eq, EnumString, EnumIter, Deserialize, Serialize)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Language {
    C,
    Cs,
    Cpp,
    Java,
    Rust,
    Python,
    Ruby,
}

#[derive(Deserialize, Serialize)]
pub struct Package {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct Dependency {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct Scenario {
    pub name: String,
    pub language: Language,
    pub description: Option<String>,
    pub code: Option<String>,
    pub origin: Option<String>,
    #[serde(default)]
    pub compile_options: Vec<String>,
    #[serde(default)]
    pub runtime_options: Vec<String>,
    pub framework: Option<String>,
    #[serde(default)]
    pub dependencides: Vec<Dependency>,
    #[serde(default)]
    pub packages: Vec<Package>,
    #[serde(skip)]
    pub target: String,
    #[serde(skip)]
    pub source: String,
}

#[derive(Debug, Error)]
pub enum ScenarioError {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML Parsing Error: {0}")]
    Yaml(#[from] serde_yml::Error),
    #[error("Missing Code Error")]
    MissingCode,
}

pub enum ScenarioResult {
    Success {
        out: String,
        err: String,
    },
    Failed {
        exit_code: i32,
        out: String,
        err: String,
    },
}

impl ScenarioResult {
    pub fn success() -> Self {
        Self::Success {
            out: String::new(),
            err: String::new(),
        }
    }

    pub fn success_with(out: String, err: String) -> Self {
        Self::Success { out, err }
    }

    pub fn failed(exit_code: i32) -> Self {
        Self::Failed {
            exit_code,
            out: String::new(),
            err: String::new(),
        }
    }

    pub fn failed_with(exit_code: i32, out: String, err: String) -> Self {
        Self::Failed {
            exit_code,
            out,
            err,
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Test {
    pub name: Option<String>,
    #[serde(default)]
    pub compile_options: Vec<String>,
    #[serde(default)]
    pub runtime_options: Vec<String>,
    #[serde(default)]
    pub arguments: Vec<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "from_base64",
        serialize_with = "to_base64"
    )]
    pub stdin: Option<Vec<u8>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "from_base64",
        serialize_with = "to_base64"
    )]
    pub expected_stdout: Option<Vec<u8>>,
}
