use crate::config::Config;
use crate::language::Language;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScenarioError {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML Parsing Error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Missing Code Error")]
    MissingCode,
}

#[derive(Debug)]
pub enum BuildResult {
    Success {
        stdout: String,
        stderr: String,
    },
    Failed {
        exit_code: Option<i32>,
        stdout: String,
        stderr: String,
    },
    Skipped,
}

#[derive(Debug)]
pub struct ExecuteResult {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Scenario {
    pub name: String,
    pub language: Language,
    pub description: Option<String>,
    // pub dependencides: Option<Vec<>>
    pub compile_options: Option<Vec<String>>,
    pub runtime_options: Option<Vec<String>>,
    pub code: Option<String>,
    #[serde(skip)]
    pub code_origin: Option<String>,
    #[serde(skip)]
    pub target: String,
    #[serde(skip)]
    pub source: String,
}

impl TryFrom<&Path> for Scenario {
    type Error = ScenarioError;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let content = fs::read_to_string(path)?;
        serde_yaml::from_str(&content).map_err(ScenarioError::Yaml)
    }
}

impl TryFrom<&str> for Scenario {
    type Error = ScenarioError;

    fn try_from(path: &str) -> Result<Self, Self::Error> {
        Self::try_from(Path::new(path))
    }
}

impl Scenario {
    pub fn scenario_path(&self) -> PathBuf {
        let code_origin = self.code_origin.as_deref().unwrap_or("human");
        Config::global()
            .base_dir
            .join("results")
            .join(code_origin)
            .join(&self.language.to_string())
            .join(&self.name)
    }

    pub fn target_path(&self) -> PathBuf {
        self.scenario_path().join(&self.language.target_file())
    }

    pub fn source_path(&self) -> PathBuf {
        self.scenario_path().join(&self.language.source_file())
    }

    pub fn build_command(&self) -> Vec<String> {
        let mut command = match self.language {
            Language::C => vec![
                "gcc".to_string(),
                self.source_path().to_string_lossy().to_string(),
                "-o".to_string(),
                self.target_path().to_string_lossy().to_string(),
                "-w".to_string(),
            ],
        };
        if let Some(ref options) = self.compile_options {
            command.extend_from_slice(options);
        }
        command
    }

    pub fn execute_command(&self) -> Vec<String> {
        let mut command = match self.language {
            Language::C => vec![self.target_path().to_string_lossy().to_string()],
        };
        if let Some(ref options) = self.runtime_options {
            command.extend_from_slice(options);
        }
        command
    }

    // pub fn clean_command(&self) -> Vec<String> {
    //     match self.language {
    //         Language::C => {
    //             vec!["rm"self.target_path().to_string_lossy().to_string()]
    //         }
    //     }
    // }

    pub fn build(&self) -> Result<BuildResult, ScenarioError> {
        let code = self.code.as_ref().ok_or(ScenarioError::MissingCode)?;
        let source_path = self.source_path();
        if let Some(parent) = source_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(source_path, code)?;
        let command = self.build_command();
        if command.is_empty() {
            return Err(ScenarioError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "No build command available for this language",
            )));
        }

        let output = Command::new(&command[0])
            .args(&command[1..])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            Ok(BuildResult::Success {
                stdout: stdout.to_string(),
                stderr: stderr.to_string(),
            })
        } else {
            Ok(BuildResult::Failed {
                exit_code: output.status.code(),
                stdout: stdout.to_string(),
                stderr: stderr.to_string(),
            })
        }
    }

    pub fn measure(&self) -> Result<ExecuteResult, ScenarioError> {
        let command = self.execute_command();
        if command.is_empty() {
            return Err(ScenarioError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "No execution command available for this language",
            )));
        }
        let output = Command::new(&command[0])
            .args(&command[1..])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        Ok(ExecuteResult {
            exit_code: output.status.code(),
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
            success: output.status.success(),
        })
    }
}
