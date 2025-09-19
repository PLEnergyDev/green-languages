use std::env;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct Config {
    pub base_dir: PathBuf,
}

impl Config {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let base_dir = env::var("BASE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| env::current_dir().unwrap());
        Ok(Config { base_dir })
    }

    pub fn global() -> &'static Config {
        static CONFIG: OnceLock<Config> = OnceLock::new();
        CONFIG.get_or_init(|| Config::new().expect("Failed to initialize config"))
    }
}
