use super::util::{lib_dir_str, results_dir, CommandEnvExt};
use super::{Language, Scenario, ScenarioError, ScenarioResult, Test};
use serde::Deserialize;
use serde_yml::Deserializer;
use std::fs::{self, File};
use std::io::{BufReader, Error, ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

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

impl TryFrom<&Path> for Scenario {
    type Error = ScenarioError;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let file = File::open(path).map_err(|e| {
            if e.kind() == ErrorKind::NotFound {
                ScenarioError::NotFound(path.to_path_buf())
            } else {
                ScenarioError::Io(e)
            }
        })?;

        let reader = BufReader::new(file);
        let mut deserializer = Deserializer::from_reader(reader);
        let first_doc = deserializer.next().ok_or_else(|| {
            ScenarioError::Io(Error::new(
                ErrorKind::InvalidData,
                "No scenario document found in file",
            ))
        })?;
        let scenario: Scenario =
            serde_yml::Value::deserialize(first_doc).and_then(serde_yml::from_value)?;
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
    pub fn scenario_dir(&self) -> PathBuf {
        results_dir()
            .join("build")
            .join(&self.language.to_string())
            .join(&self.name)
    }

    fn test_dir(&self, test: &Test) -> PathBuf {
        let test_name = test.name.as_ref().expect("test has no name");
        self.scenario_dir().join(test_name)
    }

    fn target_path(&self, test: &Test) -> PathBuf {
        self.test_dir(&test).join(&self.language.target_file())
    }

    fn source_path(&self) -> PathBuf {
        self.scenario_dir().join(&self.language.source_file())
    }

    fn stdout_path(&self, test: &Test) -> PathBuf {
        self.test_dir(test).join("stdout.txt")
    }

    pub fn test_expected_stdout_path(&self, test: &Test) -> PathBuf {
        self.test_dir(test).join("expected_stdout.txt")
    }

    pub fn scenario_expected_stdout_path(&self) -> PathBuf {
        self.scenario_dir().join("expected_stdout.txt")
    }

    fn test_stdin_path(&self, test: &Test) -> PathBuf {
        self.test_dir(test).join("stdin.txt")
    }

    fn scenario_stdin_path(&self) -> PathBuf {
        self.scenario_dir().join("stdin.txt")
    }

    pub fn exec_command(&self, test: &Test) -> Result<Vec<String>, ScenarioError> {
        let target = self.target_path(&test).to_string_lossy().to_string();
        let test_dir = self.test_dir(&test).to_string_lossy().to_string();

        match self.language {
            Language::C | Language::Cpp => Ok(vec![target]),
            Language::Cs => {
                let executable = self.test_dir(test).join("Program");
                if !executable.exists() {
                    return Err(ScenarioError::Io(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "C# executable not found",
                    )));
                }
                Ok(vec![executable.to_string_lossy().to_string()])
            }
            Language::Java => {
                let cp_flags = format!("{}:{}", lib_dir_str(), test_dir);
                Ok(vec![
                    "java".to_string(),
                    "--enable-native-access=ALL-UNNAMED".to_string(),
                    "-cp".to_string(),
                    cp_flags,
                    self.language.target_file().to_string(),
                ])
            }
            Language::Rust => {
                let test_dir = self.test_dir(test);
                let release_path = test_dir.join("release").join("program");
                let debug_path = test_dir.join("debug").join("program");
                let executable = if release_path.exists() {
                    release_path
                } else if debug_path.exists() {
                    debug_path
                } else {
                    return Err(ScenarioError::Io(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "Rust executable not found",
                    )));
                };

                Ok(vec![executable.to_string_lossy().to_string()])
            }
            Language::Python => Ok(vec![]),
            Language::Ruby => Ok(vec![]),
        }
    }

    pub fn build_command(&self, test: &Test) -> Vec<String> {
        let scenario = self.scenario_dir().to_string_lossy().to_string();
        let source = self.source_path().to_string_lossy().to_string();
        let target = self.target_path(&test).to_string_lossy().to_string();
        let test_dir = self.test_dir(&test).to_string_lossy().to_string();

        match self.language {
            Language::C => vec![
                "gcc".to_string(),
                source,
                "-o".to_string(),
                target,
                "-lmeasurements".to_string(),
            ],
            Language::Cs => vec![
                "dotnet".to_string(),
                "build".to_string(),
                scenario,
                "-p:OutputType=Exe".to_string(),
                "--output".to_string(),
                test_dir,
            ],
            Language::Cpp => vec![
                "g++".to_string(),
                source,
                "-o".to_string(),
                target,
                "-lmeasurements".to_string(),
            ],
            Language::Java => {
                let cp_flags = format!("{}:{}", lib_dir_str(), test_dir);
                vec![
                    "javac".to_string(),
                    source,
                    "-d".to_string(),
                    test_dir,
                    "-cp".to_string(),
                    cp_flags,
                ]
            }
            Language::Rust => {
                let toml_path = self
                    .scenario_dir()
                    .join("Cargo.toml")
                    .to_string_lossy()
                    .to_string();
                vec![
                    "cargo".to_string(),
                    "build".to_string(),
                    "--manifest-path".to_string(),
                    toml_path,
                    "--target-dir".to_string(),
                    test_dir,
                ]
            }
            Language::Python | Language::Ruby => vec![],
        }
    }

    pub fn build_test(
        &mut self,
        test: &mut Test,
        index: usize,
    ) -> Result<ScenarioResult, ScenarioError> {
        let code = self.code.as_ref().ok_or(ScenarioError::MissingCode)?;
        if code.trim().is_empty() {
            return Err(ScenarioError::MissingCode);
        }

        let source_path = self.source_path();
        let test_dir = self.test_dir(&test);

        fs::create_dir_all(&test_dir)?;
        fs::write(&source_path, code)?;

        if test.name.is_none() {
            test.name = Some(index.to_string());
        }
        if let Some(stdin_data) = self.stdin.take() {
            fs::write(self.scenario_stdin_path(), stdin_data)?;
        }
        if let Some(expected_stdout_data) = self.expected_stdout.take() {
            fs::write(self.scenario_expected_stdout_path(), expected_stdout_data)?;
        }

        match self.language {
            Language::Cs => self.prepare_cs_build(&test)?,
            Language::Rust => self.prepare_rust_build(&test)?,
            _ => (),
        }

        let mut command = self.build_command(&test);
        let compile_opts = test
            .compile_options
            .as_ref()
            .or(self.compile_options.as_ref());
        if let Some(opts) = compile_opts {
            for opt in opts {
                command.extend(opt.split_whitespace().map(|s| s.to_string()));
            }
        }

        let output = Command::new(&command[0])
            .args(&command[1..])
            .with_measurements_env()
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;
        let code = output.status.code().unwrap_or_default();
        let out = String::from_utf8_lossy(&output.stdout).to_string();
        let err = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            if let Some(stdin_data) = test.stdin.take() {
                fs::write(self.test_stdin_path(&test), stdin_data)?;
            }
            if let Some(expected_stdout_data) = test.expected_stdout.take() {
                fs::write(self.test_expected_stdout_path(&test), expected_stdout_data)?;
            }
            Ok(ScenarioResult::success_with(out, err))
        } else {
            test.stdin.take();
            test.expected_stdout.take();
            Ok(ScenarioResult::failed_with(code, out, err))
        }
    }

    pub fn exec_test_async(&self, test: &Test) -> Result<Child, ScenarioError> {
        match self.language {
            Language::C | Language::Cpp | Language::Rust | Language::Cs => {
                let has_runtime_opts =
                    test.runtime_options.is_some() || self.runtime_options.is_some();
                if has_runtime_opts {
                    return Err(ScenarioError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!(
                            "Runtime options are not supported for compiled language '{}'",
                            self.language
                        ),
                    )));
                }
            }
            _ => {}
        }

        let mut command = self.exec_command(&test)?;

        let runtime_opts = test
            .runtime_options
            .as_ref()
            .or(self.runtime_options.as_ref());
        if let Some(opts) = runtime_opts {
            for opt in opts {
                command.extend(opt.split_whitespace().map(|s| s.to_string()));
            }
        }

        let args = test.arguments.as_ref().or(self.arguments.as_ref());
        if let Some(arguments) = args {
            for arg in arguments {
                command.extend(arg.split_whitespace().map(|s| s.to_string()));
            }
        }

        let stdout_path = self.stdout_path(&test);
        let output_file = File::create(&stdout_path)?;
        let test_stdin_path = self.test_stdin_path(&test);
        let scenario_stdin_path = self.scenario_stdin_path();
        let stdin_config = if test_stdin_path.exists() {
            let input_file = File::open(&test_stdin_path)?;
            Stdio::from(input_file)
        } else if scenario_stdin_path.exists() {
            let input_file = File::open(&scenario_stdin_path)?;
            Stdio::from(input_file)
        } else {
            Stdio::null()
        };

        let child = Command::new(&command[0])
            .args(&command[1..])
            .stdout(Stdio::from(output_file))
            .stderr(Stdio::piped())
            .stdin(stdin_config)
            .with_measurements_env()
            .spawn()?;

        Ok(child)
    }

    pub fn verify_test(
        &self,
        test: &Test,
        iterations: usize,
    ) -> Result<ScenarioResult, ScenarioError> {
        let test_expected_stdout_path = self.test_expected_stdout_path(test);
        let scenario_expected_stdout_path = self.scenario_expected_stdout_path();

        let expected_stdout_path = if test_expected_stdout_path.exists() {
            test_expected_stdout_path
        } else if scenario_expected_stdout_path.exists() {
            scenario_expected_stdout_path
        } else {
            return Ok(ScenarioResult::success());
        };

        let expected = std::fs::read(&expected_stdout_path)?;
        let expected_len = expected.len();
        let stdout_path = self.stdout_path(test);
        let file = File::open(&stdout_path)?;
        let mut reader = BufReader::with_capacity(expected_len * 16, file);
        let mut buffer = vec![0u8; expected_len];

        for i in 0..iterations {
            match reader.read_exact(&mut buffer) {
                Ok(_) => {
                    if buffer != expected {
                        let err = format!(
                            "test '{}' got unexpected stdout for iteration {}: content unequal",
                            test.name.as_ref().unwrap_or(&"unknown".to_string()),
                            i + 1
                        );
                        return Ok(ScenarioResult::failed_with(1, String::new(), err));
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    let err = format!(
                        "test '{}' got unexpected stdout for iteration {}: output too short",
                        test.name.as_ref().unwrap_or(&"unknown".to_string()),
                        i + 1
                    );
                    return Ok(ScenarioResult::failed_with(1, String::new(), err));
                }
                Err(e) => return Err(ScenarioError::Io(e)),
            }
        }

        let mut extra = [0u8; 1];
        match reader.read(&mut extra) {
            Ok(0) => Ok(ScenarioResult::success()),
            Ok(_) => Ok(ScenarioResult::failed_with(
                1,
                "test has more output than expected".to_string(),
                String::new(),
            )),
            Err(e) => Err(ScenarioError::Io(e)),
        }
    }

    fn prepare_cs_build(&self, test: &Test) -> Result<(), ScenarioError> {
        let csproj_path = self.scenario_dir().join("Program.csproj");
        let mut dep_formatted = String::new();
        let framework = self.framework.as_ref().ok_or_else(|| {
            ScenarioError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "A .NET framework is required for C# scenarios",
            ))
        })?;

        let deps = test.dependencies.as_ref().or(self.dependencies.as_ref());

        if let Some(dependencies) = deps {
            for dep in dependencies {
                let version = dep.version.as_deref().unwrap_or("*");
                dep_formatted.push_str(&format!(
                    r#"<PackageReference Include="{}" Version="{}" />\n"#,
                    dep.name, version
                ));
            }
        }

        let csproj_content = format!(
            r#"<Project Sdk="Microsoft.NET.Sdk">
               <PropertyGroup>
                   <TargetFramework>{}</TargetFramework>
               </PropertyGroup>
               <ItemGroup>{}</ItemGroup>
           </Project>"#,
            framework, dep_formatted
        );

        fs::write(&csproj_path, csproj_content)?;
        Ok(())
    }

    fn prepare_rust_build(&self, test: &Test) -> Result<(), ScenarioError> {
        let toml_path = self.scenario_dir().join("Cargo.toml");
        let mut dep_formatted = String::new();

        let deps = test.dependencies.as_ref().or(self.dependencies.as_ref());

        if let Some(dependencies) = deps {
            for dep in dependencies {
                let version = dep.version.as_deref().unwrap_or("*");
                dep_formatted.push_str(&format!(r#"{} = "{}"\n"#, dep.name, version));
            }
        }

        let toml_content = format!(
            r#"[package]
        name = "program"
        version = "0.1.0"
        edition = "2024"

        [[bin]]
        name = "program"
        path = "main.rs"

        [dependencies]
        {}"#,
            dep_formatted
        );

        fs::write(&toml_path, toml_content)?;
        Ok(())
    }
}
