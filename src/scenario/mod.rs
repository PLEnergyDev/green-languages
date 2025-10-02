pub mod error;
pub mod result;
pub mod test;
pub mod util;

use crate::config::Config;
use crate::language::Language;
use error::ScenarioError;
use result::ScenarioResult;
use serde::{Deserialize, Serialize};
use serde_yml::Deserializer;
use std::fs::{self, File};
use std::io::{BufReader, Error, ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use test::Test;
use util::CommandEnvExt;

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
            ScenarioError::Io(Error::new(
                ErrorKind::InvalidData,
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
    fn scenario_path(&self) -> PathBuf {
        let code_origin = self.code_origin.as_deref().unwrap_or("human");
        Config::global()
            .base_dir
            .join("results")
            .join(code_origin)
            .join(&self.language.to_string())
            .join(&self.name)
    }

    fn test_path(&self, test: &Test) -> PathBuf {
        let test_id = test.id.as_ref().expect("Test has no id");
        self.scenario_path().join(test_id)
    }

    fn target_path(&self) -> PathBuf {
        self.scenario_path().join(&self.language.target_file())
    }

    fn source_path(&self) -> PathBuf {
        self.scenario_path().join(&self.language.source_file())
    }

    fn stdin_path(&self, test: &Test) -> PathBuf {
        self.test_path(test).join("stdin.txt")
    }

    fn stdout_path(&self, test: &Test) -> PathBuf {
        self.test_path(test).join("stdout.txt")
    }

    fn expected_stdout_path(&self, test: &Test) -> PathBuf {
        self.test_path(test).join("expected_stdout.txt")
    }

    fn build_test_command(&self, test: &Test) -> Vec<String> {
        let target_path = self.target_path().to_string_lossy().to_string();
        let source_path = self.source_path().to_string_lossy().to_string();
        let mut command = match self.language {
            Language::C => vec![
                "gcc".to_string(),
                source_path,
                "-o".to_string(),
                target_path,
                "-w".to_string(),
                "-literations".to_string(),
            ],
        };

        if let Some(ref options) = test.compile_options {
            command.extend_from_slice(options);
        }
        command
    }

    pub fn build_test(&self, test: &mut Test) -> Result<ScenarioResult, ScenarioError> {
        let code = self.code.as_ref().ok_or(ScenarioError::MissingCode)?;
        let test_path = self.test_path(&test);
        let source_path = self.source_path();
        fs::create_dir_all(&test_path)?;
        fs::write(&source_path, code)?;

        let command = self.build_test_command(&test);
        if command.is_empty() {
            let err_msg = format!("Build command not available for {}", self.language);
            return Err(ScenarioError::Io(Error::new(
                ErrorKind::InvalidInput,
                err_msg,
            )));
        }

        let output = Command::new(&command[0])
            .args(&command[1..])
            .with_iter_signal_env()
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

    fn exec_test_command(&self, test: &Test) -> Vec<String> {
        let target_path = self.target_path().to_string_lossy().to_string();
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

    pub fn exec_test_async(&self, test: &Test) -> Result<Child, ScenarioError> {
        let command = self.exec_test_command(&test);
        if command.is_empty() {
            let err_msg = format!("Exec command not available for {}", self.language);
            return Err(ScenarioError::Io(Error::new(
                ErrorKind::InvalidInput,
                err_msg,
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
            .with_iter_signal_env()
            .stdout(Stdio::from(output_file))
            .stderr(Stdio::piped())
            .stdin(stdin_config)
            .spawn()?;

        Ok(child)
    }

    pub fn exec_test(&self, test: &Test) -> Result<ScenarioResult, ScenarioError> {
        let child = self.exec_test_async(test)?;
        let output = child.wait_with_output()?;
        let exit_code = output.status.code().unwrap_or_default();
        let err = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(ScenarioResult::success())
        } else {
            Ok(ScenarioResult::Failed { exit_code, err })
        }
    }

    pub fn verify_test(
        &self,
        test: &Test,
        iterations: usize,
    ) -> Result<ScenarioResult, ScenarioError> {
        let expected_stdout_path = self.expected_stdout_path(test);
        if !expected_stdout_path.exists() {
            return Ok(ScenarioResult::success());
        }

        let stdout_path = self.stdout_path(test);
        let expected = std::fs::read(&expected_stdout_path)?;
        let expected_len = expected.len();
        let file = File::open(&stdout_path)?;
        let mut reader = BufReader::new(file);
        let mut buffer = vec![0u8; expected_len];

        for i in 0..iterations {
            match reader.read_exact(&mut buffer) {
                Ok(_) => {
                    if buffer != expected {
                        return Ok(ScenarioResult::Failed {
                            exit_code: 1,
                            err: format!(
                                "test '{}' got unexpected stdout for iteration {}: content unequal",
                                test.id.as_ref().unwrap_or(&"unknown".to_string()),
                                i + 1
                            ),
                        });
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    return Ok(ScenarioResult::Failed {
                        exit_code: 1,
                        err: format!(
                            "test '{}' got unexpected stdout for iteration {}: output too short",
                            test.id.as_ref().unwrap_or(&"unknown".to_string()),
                            i + 1
                        ),
                    });
                }
                Err(e) => return Err(ScenarioError::Io(e)),
            }
        }

        let mut extra = [0u8; 1];
        match reader.read(&mut extra) {
            Ok(0) => Ok(ScenarioResult::success()),
            Ok(_) => Ok(ScenarioResult::Failed {
                exit_code: 1,
                err: "test has more output than expected".to_string(),
            }),
            Err(e) => Err(ScenarioError::Io(e)),
        }
    }
}
