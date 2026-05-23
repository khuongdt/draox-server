use anyhow::{Context, Result};
use openapiv3::OpenAPI;
use std::path::Path;

pub fn load(path: &Path) -> Result<OpenAPI> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read {}", path.display()))?;

    let spec: OpenAPI = if path.extension().and_then(|e| e.to_str()) == Some("yaml")
        || path.extension().and_then(|e| e.to_str()) == Some("yml")
    {
        serde_yaml_load(&raw)?
    } else {
        serde_json::from_str(&raw).context("invalid JSON spec")?
    };

    Ok(spec)
}

fn serde_yaml_load(s: &str) -> Result<OpenAPI> {
    // Minimal YAML → JSON roundtrip via serde_json (no yaml dep required in workspace).
    // Limitation: only works for specs that are also valid JSON-serialisable after
    // serde_json::Value parsing. For real YAML support, add serde_yaml to workspace.
    let val: serde_json::Value = serde_json::from_str(s).context("spec is not valid JSON")?;
    let api: OpenAPI = serde_json::from_value(val).context("cannot parse OpenAPI spec")?;
    Ok(api)
}
