use base64::Engine;
use green::measurements::{self, MeasurementContext};
use serde::{Deserialize, Deserializer, Serializer, de};
use serde_yaml_ng::Value;
use std::ffi::CString;

pub fn java_cp() -> String {
    std::env::var("CLASSPATH").unwrap_or_default()
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

pub struct Measurement(*mut MeasurementContext);

impl Measurement {
    pub fn start(metrics: &str) -> Self {
        let c = CString::new(metrics).expect("metrics string contains null byte");
        let ptr = measurements::measure_start(c.as_ptr());
        Self(ptr)
    }
}

impl Drop for Measurement {
    fn drop(&mut self) {
        if !self.0.is_null() {
            measurements::measure_stop(self.0);
        }
    }
}
