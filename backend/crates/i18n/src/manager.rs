use std::collections::HashMap;
use std::sync::Arc;
use dashmap::DashMap;
use thiserror::Error;
use tracing::warn;
use crate::locale::{Locale, DEFAULT_LOCALE};
use crate::template::render;

#[derive(Debug, Error)]
pub enum I18nError {
    #[error("locale not loaded: {0}")]
    LocaleNotLoaded(String),
    #[error("message key not found: {0}")]
    KeyNotFound(String),
}

/// A flat mapping of message key → template string for one locale.
pub type MessageMap = HashMap<String, String>;

/// Central i18n manager.
///
/// Load catalogs via [`I18nManager::load_catalog`]. Translate via [`I18nManager::t`].
/// Falls back to base language, then to the default locale.
#[derive(Clone, Default)]
pub struct I18nManager {
    catalogs: Arc<DashMap<String, MessageMap>>,
    default_locale: String,
}

impl I18nManager {
    pub fn new(default_locale: impl Into<String>) -> Self {
        Self {
            catalogs: Arc::default(),
            default_locale: default_locale.into(),
        }
    }

    pub fn with_default_english() -> Self {
        Self::new(DEFAULT_LOCALE)
    }

    /// Register a message catalog for a locale tag (e.g. `"en"`, `"vi-VN"`).
    pub fn load_catalog(&self, locale: impl Into<String>, messages: MessageMap) {
        self.catalogs.insert(locale.into(), messages);
    }

    /// Load catalog from embedded JSON string (for bundled locales).
    pub fn load_catalog_json(
        &self,
        locale: impl Into<String>,
        json: &str,
    ) -> Result<(), serde_json::Error> {
        let messages: MessageMap = serde_json::from_str(json)?;
        self.load_catalog(locale, messages);
        Ok(())
    }

    /// Translate `key` for `locale`, substituting `vars` into the template.
    /// Fallback chain: exact locale → base language → default locale → key itself.
    pub fn t(
        &self,
        locale: &Locale,
        key: &str,
        vars: &HashMap<String, String>,
    ) -> String {
        let template = self
            .find_template(locale.as_str().as_str(), key)
            .or_else(|| self.find_template(&locale.language, key))
            .or_else(|| self.find_template(&self.default_locale, key))
            .unwrap_or_else(|| {
                warn!(key, locale = %locale, "i18n key not found");
                key.to_string()
            });
        render(&template, vars)
    }

    /// Simple translate with no substitution variables.
    pub fn tr(&self, locale: &Locale, key: &str) -> String {
        self.t(locale, key, &HashMap::new())
    }

    fn find_template(&self, locale_str: &str, key: &str) -> Option<String> {
        self.catalogs
            .get(locale_str)
            .and_then(|cat| cat.get(key).cloned())
    }

    pub fn supported_locales(&self) -> Vec<String> {
        let mut locales: Vec<_> = self.catalogs.iter().map(|e| e.key().clone()).collect();
        locales.sort();
        locales
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_manager() -> I18nManager {
        let mgr = I18nManager::with_default_english();
        let mut en = MessageMap::new();
        en.insert("greeting".into(), "Hello, {name}!".into());
        en.insert("farewell".into(), "Goodbye!".into());
        let mut vi = MessageMap::new();
        vi.insert("greeting".into(), "Xin chào, {name}!".into());
        mgr.load_catalog("en", en);
        mgr.load_catalog("vi", vi);
        mgr
    }

    #[test]
    fn test_translate_with_vars() {
        let mgr = make_manager();
        let locale: Locale = "vi".parse().unwrap();
        let vars = crate::i18n_vars!("name" => "An");
        assert_eq!(mgr.t(&locale, "greeting", &vars), "Xin chào, An!");
    }

    #[test]
    fn test_fallback_to_default_locale() {
        let mgr = make_manager();
        let locale: Locale = "vi".parse().unwrap();
        // "farewell" only in English — should fallback
        assert_eq!(mgr.tr(&locale, "farewell"), "Goodbye!");
    }

    #[test]
    fn test_missing_key_returns_key() {
        let mgr = make_manager();
        let locale: Locale = "en".parse().unwrap();
        assert_eq!(mgr.tr(&locale, "nonexistent.key"), "nonexistent.key");
    }
}
