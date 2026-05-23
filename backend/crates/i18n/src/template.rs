use std::collections::HashMap;

/// Substitute `{key}` placeholders in `template` with values from `vars`.
/// Unknown keys are left as-is.
pub fn render(template: &str, vars: &HashMap<String, String>) -> String {
    let mut result = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            let key: String = chars.by_ref().take_while(|&ch| ch != '}').collect();
            if let Some(val) = vars.get(&key) {
                result.push_str(val);
            } else {
                // Preserve unknown placeholder
                result.push('{');
                result.push_str(&key);
                result.push('}');
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Convenience: build `vars` from alternating key/value pairs.
#[macro_export]
macro_rules! i18n_vars {
    ($($k:expr => $v:expr),* $(,)?) => {{
        let mut m = ::std::collections::HashMap::new();
        $(m.insert($k.to_string(), $v.to_string());)*
        m
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_render_simple() {
        let mut vars = HashMap::new();
        vars.insert("name".into(), "Alice".into());
        assert_eq!(render("Hello, {name}!", &vars), "Hello, Alice!");
    }

    #[test]
    fn test_render_multiple() {
        let mut vars = HashMap::new();
        vars.insert("user".into(), "Bob".into());
        vars.insert("count".into(), "3".into());
        assert_eq!(render("{user} has {count} messages", &vars), "Bob has 3 messages");
    }

    #[test]
    fn test_unknown_placeholder_preserved() {
        let vars = HashMap::new();
        assert_eq!(render("Hello, {name}!", &vars), "Hello, {name}!");
    }
}
