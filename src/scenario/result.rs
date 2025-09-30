#[derive(Debug)]
pub enum ScenarioResult {
    Success { out: String, err: String },
    Failed { exit_code: i32, err: String },
}

impl ScenarioResult {
    pub fn success() -> Self {
        ScenarioResult::Success {
            out: String::new(),
            err: String::new(),
        }
    }

    pub fn failed() -> Self {
        ScenarioResult::Failed {
            exit_code: 1,
            err: String::new(),
        }
    }
}
