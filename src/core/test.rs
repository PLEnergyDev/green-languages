use super::Test;
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

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
