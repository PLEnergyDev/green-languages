use super::Test;
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

impl Test {
    pub fn iterate_from_file(
        path: &Path,
    ) -> Result<impl Iterator<Item = Result<Test, serde_yaml_ng::Error>>, std::io::Error> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let deserializer = serde_yaml_ng::Deserializer::from_reader(reader);
        Ok(deserializer
            .into_iter()
            .skip(1)
            .map(|document| Test::deserialize(document)))
    }
}

impl Default for Test {
    fn default() -> Self {
        Self {
            name: Some("1".to_string()),
            compile_options: None,
            runtime_options: None,
            dependencies: None,
            settings: None,
            affinity: None,
            niceness: None,
            arguments: None,
            stdin: None,
            expected_stdout: None,
        }
    }
}

