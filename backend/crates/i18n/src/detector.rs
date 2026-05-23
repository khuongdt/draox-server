use crate::locale::{Locale, DEFAULT_LOCALE};

/// Parse the `Accept-Language` HTTP header (RFC 7231) and return locales
/// sorted by quality value (descending).
///
/// Example header: `en-US,en;q=0.9,vi;q=0.8`
pub fn detect_from_header(header: &str) -> Vec<Locale> {
    let mut tagged: Vec<(Locale, f32)> = header
        .split(',')
        .filter_map(|part| {
            let mut segments = part.trim().splitn(2, ";q=");
            let tag = segments.next()?.trim();
            let q: f32 = segments
                .next()
                .and_then(|q| q.trim().parse().ok())
                .unwrap_or(1.0);
            tag.parse::<Locale>().ok().map(|l| (l, q))
        })
        .collect();
    tagged.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    tagged.into_iter().map(|(l, _)| l).collect()
}

/// Choose the best supported locale given the client's preferences.
/// Falls back to `default_locale` (usually "en").
/// RFC 4647 basic filtering: for each preference (highest q first) try exact
/// match then language-only fallback before moving to the next preference.
pub fn choose_locale<'a>(
    preferences: &[Locale],
    supported: &'a [Locale],
    default_locale: &'a Locale,
) -> &'a Locale {
    for pref in preferences {
        if let Some(found) = supported.iter().find(|s| *s == pref) {
            return found;
        }
        let base = pref.base();
        if let Some(found) = supported.iter().find(|s| s.language == base.language) {
            return found;
        }
    }
    default_locale
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_accept_language() {
        let locales = detect_from_header("en-US,en;q=0.9,vi;q=0.8");
        assert_eq!(locales[0].as_str(), "en-US");
        assert_eq!(locales[1].language, "en");
        assert_eq!(locales[2].language, "vi");
    }

    #[test]
    fn test_choose_locale_exact() {
        let prefs: Vec<Locale> = vec!["vi-VN".parse().unwrap(), "en".parse().unwrap()];
        let supported: Vec<Locale> = vec!["en".parse().unwrap(), "vi".parse().unwrap()];
        let default = supported.last().unwrap();
        let chosen = choose_locale(&prefs, &supported, default);
        assert_eq!(chosen.language, "vi");
    }

    #[test]
    fn test_choose_locale_fallback_to_default() {
        let prefs: Vec<Locale> = vec!["zh".parse().unwrap()];
        let supported: Vec<Locale> = vec!["en".parse().unwrap()];
        let default = &supported[0];
        let chosen = choose_locale(&prefs, &supported, default);
        assert_eq!(chosen.language, "en");
    }
}
