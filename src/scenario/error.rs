use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScenarioError {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML Parsing Error: {0}")]
    Yaml(#[from] serde_yml::Error),
    #[error("Missing Code Error")]
    MissingCode,
}
