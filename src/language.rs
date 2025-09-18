use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Display, EnumString, EnumIter, Deserialize, Serialize,
)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Language {
    C,
    // Cs,
    // Cpp,
    // Rust,
    // Python,
    // Java,
}

pub fn supported_languages() -> impl Iterator<Item = Language> {
    Language::iter()
}

pub fn is_supported(language: &str) -> bool {
    language.parse::<Language>().is_ok()
}

impl Language {
    pub fn source_file(&self) -> &'static str {
        match self {
            Language::C => "main.c",
            // Language::Cs => "Program.cs",
            // Language::Cpp => "main.cpp",
            // Language::Rust => "main.rs",
            // Language::Python => "main.py",
            // Language::Java => "Program.java",
        }
    }

    pub fn target_file(&self) -> &'static str {
        match self {
            Language::C => "main",
            // Language::Cs => "Program",
            // Language::Cpp => "main",
            // Language::Rust => "main",
            // Language::Python => "main.py",
            // Language::Java => "Program",
        }
    }

    pub fn is_compiled(&self) -> bool {
        match self {
            // Language::C | Language::Cs | Language::Cpp | Language::Rust | Language::Java => true,
            // Language::Python => false,
            Language::C => false,
        }
    }
}
