use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct Config {
    pub lib_dir: PathBuf,
}

impl Config {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let lib_dir = option_env!("GL_LIB_DIR").ok_or(
            "GL_LIB_DIR was not set during compilation. Please reinstall the application.",
        )?;

        let lib_dir = PathBuf::from(lib_dir);

        if !lib_dir.exists() {
            return Err(format!("Library directory does not exist: {}", lib_dir.display()).into());
        }

        Ok(Config { lib_dir })
    }

    pub fn global() -> &'static Config {
        static CONFIG: OnceLock<Config> = OnceLock::new();
        CONFIG.get_or_init(|| Config::new().expect("Failed to initialize config"))
    }
}
