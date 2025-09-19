#[derive(Debug)]
pub enum ScenarioResult {
    Success {
        stdout: Option<String>,
        stderr: Option<String>,
    },
    Failed {
        exit_code: Option<i32>,
        stdout: Option<String>,
        stderr: Option<String>,
    },
    Skipped,
}

impl ScenarioResult {
    pub fn success() -> Self {
        ScenarioResult::Success {
            stdout: None,
            stderr: None,
        }
    }

    pub fn success_with_output(stdout: String) -> Self {
        ScenarioResult::Success {
            stdout: Some(stdout),
            stderr: None,
        }
    }

    pub fn success_full(stdout: String, stderr: String) -> Self {
        ScenarioResult::Success {
            stdout: Some(stdout),
            stderr: Some(stderr),
        }
    }

    pub fn failed(exit_code: i32) -> Self {
        ScenarioResult::Failed {
            exit_code: Some(exit_code),
            stdout: None,
            stderr: None,
        }
    }

    pub fn failed_unknown() -> Self {
        ScenarioResult::Failed {
            exit_code: None,
            stdout: None,
            stderr: None,
        }
    }

    pub fn failed_full(exit_code: Option<i32>, stdout: String, stderr: String) -> Self {
        ScenarioResult::Failed {
            exit_code,
            stdout: Some(stdout),
            stderr: Some(stderr),
        }
    }

    pub fn skipped() -> Self {
        ScenarioResult::Skipped
    }
}
