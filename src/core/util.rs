use crate::config::Config;
use std::env;
use std::path::PathBuf;
use std::process::Command;

fn iterations_path() -> PathBuf {
    Config::global().base_dir.join("iterations").join("lib")
}

pub trait CommandEnvExt {
    fn with_iterations_env(&mut self) -> &mut Self;
}

impl CommandEnvExt for Command {
    fn with_iterations_env(&mut self) -> &mut Self {
        let iterations_path_str = iterations_path();
        let extend_path_var = |var_name: &str| {
            let current = env::var(var_name).unwrap_or_default();
            format!("{}:{}", iterations_path_str.to_string_lossy(), current)
        };

        self.env("LIBRARY_PATH", extend_path_var("LIBRARY_PATH"))
            .env("LD_LIBRARY_PATH", extend_path_var("LD_LIBRARY_PATH"))
            .env("CPATH", extend_path_var("CPATH"))
    }
}
