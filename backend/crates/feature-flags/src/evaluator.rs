use std::collections::HashMap;
use crate::flag::{ConditionOp, FeatureFlag, FlagCondition, FlagRule, FlagValue};

/// Evaluation context supplied by the caller.
/// Keys are arbitrary attribute names; values are JSON.
#[derive(Debug, Clone, Default)]
pub struct EvalContext {
    pub attributes: HashMap<String, serde_json::Value>,
}

impl EvalContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.attributes.get(key)
    }
}

/// Stateless flag evaluator.
pub struct FlagEvaluator;

impl FlagEvaluator {
    pub fn evaluate(flag: &FeatureFlag, ctx: &EvalContext) -> FlagValue {
        // A disabled flag is always "off", regardless of default_value or rules.
        if !flag.enabled {
            return FlagValue::Bool(false);
        }
        for rule in &flag.rules {
            if Self::rule_matches(rule, ctx) {
                if let Some(pct) = rule.rollout_percentage {
                    if !Self::in_rollout(ctx, flag.key.as_str(), pct) {
                        continue;
                    }
                }
                return rule.value.clone();
            }
        }
        flag.default_value.clone()
    }

    fn rule_matches(rule: &FlagRule, ctx: &EvalContext) -> bool {
        rule.conditions.iter().all(|c| Self::condition_matches(c, ctx))
    }

    fn condition_matches(cond: &FlagCondition, ctx: &EvalContext) -> bool {
        let attr = match ctx.get(&cond.attribute) {
            Some(v) => v,
            None => return false,
        };
        match &cond.operator {
            ConditionOp::Equals => attr == &cond.value,
            ConditionOp::NotEquals => attr != &cond.value,
            ConditionOp::In => cond
                .value
                .as_array()
                .map(|arr| arr.contains(attr))
                .unwrap_or(false),
            ConditionOp::Contains => {
                let haystack = attr.as_str().unwrap_or("");
                let needle = cond.value.as_str().unwrap_or("");
                haystack.contains(needle)
            }
            ConditionOp::GreaterThan => {
                let a = attr.as_f64().unwrap_or(f64::NAN);
                let b = cond.value.as_f64().unwrap_or(f64::NAN);
                a > b
            }
            ConditionOp::LessThan => {
                let a = attr.as_f64().unwrap_or(f64::NAN);
                let b = cond.value.as_f64().unwrap_or(f64::NAN);
                a < b
            }
        }
    }

    /// Deterministic percentage rollout using FNV hash of `user_id + flag_key`.
    fn in_rollout(ctx: &EvalContext, flag_key: &str, pct: u8) -> bool {
        let user_id = ctx
            .get("user_id")
            .and_then(|v| v.as_str())
            .unwrap_or("anonymous");
        let seed = format!("{user_id}:{flag_key}");
        let hash = fnv1a_32(seed.as_bytes()) as u64;
        (hash % 100) < pct as u64
    }
}

fn fnv1a_32(data: &[u8]) -> u32 {
    const OFFSET: u32 = 2_166_136_261;
    const PRIME: u32 = 16_777_619;
    data.iter().fold(OFFSET, |acc, &b| (acc ^ b as u32).wrapping_mul(PRIME))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flag::{ConditionOp, FlagCondition, FlagRule, FeatureFlag, FlagValue};

    fn make_flag(default: bool, rules: Vec<FlagRule>) -> FeatureFlag {
        FeatureFlag {
            key: "test_flag".into(),
            description: "".into(),
            enabled: true,
            rules,
            default_value: FlagValue::Bool(default),
        }
    }

    #[test]
    fn test_default_value_when_no_rules() {
        let flag = make_flag(false, vec![]);
        let ctx = EvalContext::new();
        assert_eq!(FlagEvaluator::evaluate(&flag, &ctx), FlagValue::Bool(false));
    }

    #[test]
    fn test_rule_match_equals() {
        let flag = make_flag(false, vec![FlagRule {
            conditions: vec![FlagCondition {
                attribute: "plan".into(),
                operator: ConditionOp::Equals,
                value: serde_json::json!("pro"),
            }],
            value: FlagValue::Bool(true),
            rollout_percentage: None,
        }]);
        let ctx = EvalContext::new().with("plan", "pro");
        assert_eq!(FlagEvaluator::evaluate(&flag, &ctx), FlagValue::Bool(true));
        let ctx2 = EvalContext::new().with("plan", "free");
        assert_eq!(FlagEvaluator::evaluate(&flag, &ctx2), FlagValue::Bool(false));
    }

    #[test]
    fn test_disabled_flag_returns_false() {
        let mut flag = make_flag(true, vec![]);
        flag.enabled = false;
        let ctx = EvalContext::new();
        // Disabled flags are always off, regardless of default_value.
        assert_eq!(FlagEvaluator::evaluate(&flag, &ctx), FlagValue::Bool(false));
    }

    #[test]
    fn test_rollout_deterministic() {
        let flag = make_flag(false, vec![FlagRule {
            conditions: vec![],
            value: FlagValue::Bool(true),
            rollout_percentage: Some(50),
        }]);
        // Same user always gets same result
        let ctx = EvalContext::new().with("user_id", "user-abc");
        let first = FlagEvaluator::evaluate(&flag, &ctx);
        let second = FlagEvaluator::evaluate(&flag, &ctx);
        assert_eq!(first, second);
    }
}
