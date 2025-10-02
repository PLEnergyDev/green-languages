use base64::Engine;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Test {
    pub id: Option<String>,
    pub compile_options: Option<Vec<String>>,
    pub runtime_options: Option<Vec<String>>,
    pub arguments: Option<Vec<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "from_base64",
        serialize_with = "to_base64"
    )]
    pub stdin: Option<Vec<u8>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "from_base64",
        serialize_with = "to_base64"
    )]
    pub expected_stdout: Option<Vec<u8>>,
}

impl Test {
    pub fn iterate_from_file(
        path: &Path,
    ) -> Result<impl Iterator<Item = Result<Test, serde_yml::Error>>, std::io::Error> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let deserializer = serde_yml::Deserializer::from_reader(reader);
        Ok(deserializer
            .into_iter()
            .skip(1)
            .map(|document| Test::deserialize(document)))
    }
}
