use crate::config::Config;
use std::env;
use std::path::PathBuf;
use std::process::Command;

pub trait CommandEnvExt {
    fn with_iterations_env(&mut self) -> &mut Self;
}

impl CommandEnvExt for Command {
    fn with_iterations_env(&mut self) -> &mut Self {
        let extend_path_var = |var_name: &str| {
            let current = env::var(var_name).unwrap_or_default();
            format!("{}:{}", iterations_dir_str(), current)
        };

        self.env("LIBRARY_PATH", extend_path_var("LIBRARY_PATH"))
            .env("LD_LIBRARY_PATH", extend_path_var("LD_LIBRARY_PATH"))
            .env("CPATH", extend_path_var("CPATH"))
    }
}


pub fn iterations_dir() -> PathBuf {
    Config::global().base_dir.join("iterations").join("lib")
}

pub fn iterations_dir_str() -> String {
    iterations_dir().to_string_lossy().to_string()
}

