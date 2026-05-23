use std::fmt;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("invalid locale string: {0}")]
pub struct LocaleParseError(String);

/// BCP-47-lite locale: `language[-region]`, e.g. `en`, `en-US`, `vi-VN`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Locale {
    pub language: String,
    pub region: Option<String>,
}

impl Locale {
    pub fn new(language: impl Into<String>) -> Self {
        Self { language: language.into(), region: None }
    }

    pub fn with_region(language: impl Into<String>, region: impl Into<String>) -> Self {
        Self {
            language: language.into(),
            region: Some(region.into()),
        }
    }

    /// Return the base language locale (strips region), used for fallback lookup.
    pub fn base(&self) -> Locale {
        Locale::new(self.language.clone())
    }

    pub fn as_str(&self) -> String {
        match &self.region {
            Some(r) => format!("{}-{}", self.language, r),
            None => self.language.clone(),
        }
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Locale {
    type Err = LocaleParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(LocaleParseError(s.to_string()));
        }
        let parts: Vec<&str> = s.splitn(2, |c| c == '-' || c == '_').collect();
        let lang = parts[0].to_lowercase();
        if lang.is_empty() || !lang.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(LocaleParseError(s.to_string()));
        }
        let region = parts.get(1).map(|r| r.to_uppercase());
        Ok(Locale { language: lang, region })
    }
}

/// Default server locale.
pub const DEFAULT_LOCALE: &str = "en";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let l: Locale = "en".parse().unwrap();
        assert_eq!(l.language, "en");
        assert!(l.region.is_none());
    }

    #[test]
    fn test_parse_with_region() {
        let l: Locale = "en-US".parse().unwrap();
        assert_eq!(l.language, "en");
        assert_eq!(l.region.as_deref(), Some("US"));
    }

    #[test]
    fn test_base_strips_region() {
        let l: Locale = "vi-VN".parse().unwrap();
        assert_eq!(l.base(), Locale::new("vi"));
    }
}
