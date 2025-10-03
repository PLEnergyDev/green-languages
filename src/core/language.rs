use super::Language;
use strum::IntoEnumIterator;

impl Language {
    pub fn supported_languages() -> impl Iterator<Item = Self> {
        Self::iter()
    }

    pub fn is_supported(language: &str) -> bool {
        language.parse::<Self>().is_ok()
    }

    pub fn source_file(&self) -> &'static str {
        match self {
            Language::C => "main.c",
            Language::Cs => "Program.cs",
            Language::Cpp => "main.cpp",
            Language::Java => "Program.java",
            Language::Rust => "main.rs",
            Language::Python => "main.py",
            Language::Ruby => "main.rb",
        }
    }

    pub fn target_file(&self) -> &'static str {
        match self {
            Language::C => "main",
            Language::Cs => "Program",
            Language::Cpp => "main",
            Language::Java => "Program",
            Language::Rust => "main",
            Language::Python => "main.py",
            Language::Ruby => "main.rb",
        }
    }

    pub fn is_compiled(&self) -> bool {
        match self {
            Language::C | Language::Cs | Language::Cpp | Language::Rust | Language::Java => true,
            Language::Python | Language::Ruby => false,
        }
    }
}
