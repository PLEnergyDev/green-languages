use crate::config::Config;
use base64::Engine;
use serde::{de, Deserialize, Deserializer, Serializer};
use serde_yml::Value;
use std::env;
use std::path::PathBuf;
use std::process::Command;

pub trait CommandEnvExt {
    fn with_measurements_env(&mut self) -> &mut Self;
}

impl CommandEnvExt for Command {
    fn with_measurements_env(&mut self) -> &mut Self {
        let lib_dir = lib_dir_str();
        let extend_path_var = |var_name: &str| {
            env::var(var_name)
                .map(|current| format!("{}:{}", current, lib_dir))
                .unwrap_or_else(|_| lib_dir.to_string())
        };

        let library_path = extend_path_var("GL_LIBRARY_PATH");
        let ld_library_path = extend_path_var("GL_LD_LIBRARY_PATH");
        let cpath = extend_path_var("GL_CPATH");

        self.env("LIBRARY_PATH", library_path)
            .env("LD_LIBRARY_PATH", ld_library_path)
            .env("CPATH", cpath)
    }
}

pub fn lib_dir_str() -> String {
    Config::global().lib_dir.to_string_lossy().to_string()
}

pub fn results_dir() -> PathBuf {
    PathBuf::from(".").join("results")
}

pub fn deserialize_args<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    let values: Option<Vec<Value>> = Option::deserialize(deserializer)?;
    Ok(values.map(|vals| {
        vals.into_iter()
            .map(|v| match v {
                Value::String(s) => s,
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                _ => String::new(),
            })
            .collect()
    }))
}

pub fn from_base64<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    match opt {
        Some(s) => {
            let cleaned: String = s.chars().filter(|c| !c.is_ascii_whitespace()).collect();
            base64::engine::general_purpose::STANDARD
                .decode(cleaned.as_bytes())
                .map(Some)
                .map_err(|e| de::Error::custom(format!("invalid base64: {}", e)))
        }
        None => Ok(None),
    }
}

pub fn to_base64<S>(value: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(bytes) => {
            let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
            serializer.serialize_str(&encoded)
        }
        None => serializer.serialize_none(),
    }
}
