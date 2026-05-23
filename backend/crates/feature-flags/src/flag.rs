use serde::{Deserialize, Serialize};

/// The resolved value of a feature flag.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FlagValue {
    Bool(bool),
    String(String),
    Number(f64),
    Json(serde_json::Value),
}

impl FlagValue {
    pub fn as_bool(&self) -> Option<bool> {
        if let FlagValue::Bool(b) = self { Some(*b) } else { None }
    }
    pub fn as_str(&self) -> Option<&str> {
        if let FlagValue::String(s) = self { Some(s) } else { None }
    }
}

/// A targeting condition checked against an [`EvalContext`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagCondition {
    /// Context attribute key, e.g. `"user_id"`, `"plan"`, `"clan_id"`.
    pub attribute: String,
    pub operator: ConditionOp,
    /// The value to compare against.
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOp {
    Equals,
    NotEquals,
    In,        // value is JSON array
    Contains,  // string contains
    GreaterThan,
    LessThan,
}

/// A targeting rule: all conditions must match (AND), returns `value` if matched.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagRule {
    pub conditions: Vec<FlagCondition>,
    pub value: FlagValue,
    /// Optional percentage rollout (0–100). If set, only applied to that
    /// percentage of users (hash of user_id mod 100).
    pub rollout_percentage: Option<u8>,
}

/// A complete feature flag definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlag {
    /// Unique flag key, e.g. `"new_chat_ui"`.
    pub key: String,
    pub description: String,
    /// Whether this flag is enabled at all.
    pub enabled: bool,
    /// Ordered targeting rules. First match wins.
    pub rules: Vec<FlagRule>,
    /// Default value returned when no rule matches (or flag is disabled).
    pub default_value: FlagValue,
}

impl FeatureFlag {
    pub fn new_bool(key: impl Into<String>, default: bool) -> Self {
        Self {
            key: key.into(),
            description: String::new(),
            enabled: true,
            rules: vec![],
            default_value: FlagValue::Bool(default),
        }
    }
}
