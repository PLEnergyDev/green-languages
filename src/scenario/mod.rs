pub mod error;
pub mod result;

use crate::config::Config;
use crate::language::Language;
use crate::scenario::error::ScenarioError;
use crate::scenario::result::ScenarioResult;
use crate::test::Test;
use serde::{Deserialize, Serialize};
use serde_yml::Deserializer;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Scenario {
    pub name: String,
    pub language: Language,
    pub description: Option<String>,
    // pub dependencides: Option<Vec<>>
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
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut deserializer = Deserializer::from_reader(reader);
        let first_doc = deserializer.next().ok_or_else(|| {
            ScenarioError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "No scenario document found in file",
            ))
        })?;
        let scenario: Scenario = serde_yml::Value::deserialize(first_doc)
            .and_then(serde_yml::from_value)
            .map_err(ScenarioError::Yaml)?;
        Ok(scenario)
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

    pub fn test_path(&self, test: &Test) -> PathBuf {
        let test_id = test.id.as_ref().expect("Test has no id");
        self.scenario_path().join(test_id)
    }

    pub fn target_path(&self, test: &Test) -> PathBuf {
        self.test_path(test).join(&self.language.target_file())
    }

    pub fn source_path(&self, test: &Test) -> PathBuf {
        self.test_path(test).join(&self.language.source_file())
    }

    pub fn stdin_path(&self, test: &Test) -> PathBuf {
        self.test_path(test).join("stdin.txt")
    }

    pub fn stdout_path(&self, test: &Test) -> PathBuf {
        self.test_path(test).join("stdout.txt")
    }

    pub fn expected_stdout_path(&self, test: &Test) -> PathBuf {
        self.test_path(test).join("expected_stdout.txt")
    }

    pub fn build_test_command(&self, test: &Test) -> Vec<String> {
        let target_path = self.target_path(&test).to_string_lossy().to_string();
        let source_path = self.source_path(&test).to_string_lossy().to_string();
        let mut command = match self.language {
            Language::C => vec![
                "gcc".to_string(),
                source_path,
                "-o".to_string(),
                target_path,
                "-w".to_string(),
            ],
        };

        if let Some(ref options) = test.compile_options {
            command.extend_from_slice(options);
        }

        command
    }

    pub fn build_test(&self, test: &mut Test) -> Result<ScenarioResult, ScenarioError> {
        let code = self.code.as_ref().ok_or(ScenarioError::MissingCode)?;
        let source_path = self.source_path(&test);
        if let Some(parent) = source_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&source_path, code)?;

        let command = self.build_test_command(&test);
        if command.is_empty() {
            return Err(ScenarioError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Build command not available for this language",
            )));
        }

        let output = Command::new(&command[0])
            .args(&command[1..])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;
        let exit_code = output.status.code().unwrap_or_default();
        let out = String::from_utf8_lossy(&output.stdout).to_string();
        let err = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            if let Some(ref stdin_data) = test.stdin.take() {
                fs::write(self.stdin_path(&test), stdin_data)?;
            }
            if let Some(ref expected_stdout_data) = test.expected_stdout.take() {
                fs::write(self.expected_stdout_path(&test), expected_stdout_data)?;
            }
            Ok(ScenarioResult::Success { out, err })
        } else {
            Ok(ScenarioResult::Failed { exit_code, err })
        }
    }

    pub fn exec_test_command(&self, test: &Test) -> Vec<String> {
        let target_path = self.target_path(test).to_string_lossy().to_string();
        let mut command = match self.language {
            Language::C => vec![target_path],
        };

        if let Some(ref options) = test.runtime_options {
            command.extend_from_slice(options);
        }

        if let Some(ref args) = test.arguments {
            command.extend_from_slice(args);
        }

        command
    }

    pub fn exec_test(&self, test: &Test) -> Result<ScenarioResult, ScenarioError> {
        let command = self.exec_test_command(&test);
        if command.is_empty() {
            return Err(ScenarioError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Run command not available for this language",
            )));
        }

        let stdout_path = self.stdout_path(&test);
        let output_file = File::create(&stdout_path)?;

        let stdin_config = if test.stdin.is_some() {
            let stdin_path = self.stdin_path(&test);
            let input_file = File::open(&stdin_path)?;
            Stdio::from(input_file)
        } else {
            Stdio::null()
        };

        let child = Command::new(&command[0])
            .args(&command[1..])
            .stdout(Stdio::from(output_file))
            .stderr(Stdio::piped())
            .stdin(stdin_config)
            .spawn()?;

        let output = child.wait_with_output()?;
        let exit_code = output.status.code().unwrap_or_default();
        let err = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(ScenarioResult::success())
        } else {
            Ok(ScenarioResult::Failed { exit_code, err })
        }
    }

    pub fn verify_test(&self, test: &Test) -> Result<ScenarioResult, ScenarioError> {
        let expected_stdout_path = self.expected_stdout_path(test);
        if !expected_stdout_path.exists() {
            return Ok(ScenarioResult::success());
        }

        let stdout_path = self.stdout_path(test);
        let actual_output =
            std::fs::read_to_string(&stdout_path).map_err(|e| ScenarioError::Io(e))?;
        let expected_output =
            std::fs::read_to_string(&expected_stdout_path).map_err(|e| ScenarioError::Io(e))?;

        if actual_output.trim() == expected_output.trim() {
            Ok(ScenarioResult::success())
        } else {
            Ok(ScenarioResult::Failed {
                exit_code: 1,
                err: format!(
                    "Output Mismatch\nExpected: {}\nActual: {}",
                    expected_output.trim(),
                    actual_output.trim()
                ),
            })
        }
    }
}
