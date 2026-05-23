use plugin_sdk::manifest::PluginManifest;
use server_core::{Error, Result};
use serde::{Deserialize, Serialize};

/// Metadata about a .dxp (Draox Plugin) package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxpPackage {
    pub manifest: PluginManifest,
    pub signature: Option<Vec<u8>>,
    pub wasm_bytes: Option<Vec<u8>>,
    pub assets: Vec<PackageAsset>,
}

/// An asset file in the package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageAsset {
    pub path: String,
    pub size: u64,
}

impl DxpPackage {
    /// Parse a package from its manifest TOML and optional binary content.
    pub fn from_manifest(manifest_toml: &str) -> Result<Self> {
        let manifest = PluginManifest::from_toml(manifest_toml)?;
        Ok(Self {
            manifest,
            signature: None,
            wasm_bytes: None,
            assets: Vec::new(),
        })
    }

    /// Get the plugin ID from the manifest.
    pub fn plugin_id(&self) -> server_core::PluginId {
        self.manifest.plugin_id()
    }

    /// Check if this is a WASM plugin.
    pub fn is_wasm(&self) -> bool {
        self.manifest.plugin_type == "wasm"
    }

    /// Check if the package is signed.
    pub fn is_signed(&self) -> bool {
        self.signature.is_some()
    }

    /// Set the package signature.
    pub fn set_signature(&mut self, signature: Vec<u8>) {
        self.signature = Some(signature);
    }

    /// Validate the package structure.
    pub fn validate(&self) -> Result<()> {
        self.manifest.validate()?;

        if self.is_wasm() && self.wasm_bytes.is_none() {
            return Err(Error::Plugin {
                plugin_id: self.manifest.id.clone(),
                message: "WASM plugin package missing wasm module".to_string(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_MANIFEST: &str = r#"
id = "io.draox.test-pkg"
name = "Test Package"
version = "1.0.0"
author = "Test"
type = "builtin"
"#;

    #[test]
    fn test_from_manifest() {
        let pkg = DxpPackage::from_manifest(TEST_MANIFEST).unwrap();
        assert_eq!(pkg.manifest.name, "Test Package");
        assert!(!pkg.is_wasm());
        assert!(!pkg.is_signed());
    }

    #[test]
    fn test_validate_builtin() {
        let pkg = DxpPackage::from_manifest(TEST_MANIFEST).unwrap();
        assert!(pkg.validate().is_ok());
    }

    #[test]
    fn test_validate_wasm_missing_bytes() {
        let toml = r#"
id = "io.draox.wasm-pkg"
name = "WASM Pkg"
version = "1.0.0"
author = "Test"
type = "wasm"

[wasm]
module = "plugin.wasm"
"#;
        let pkg = DxpPackage::from_manifest(toml).unwrap();
        assert!(pkg.validate().is_err());
    }

    #[test]
    fn test_set_signature() {
        let mut pkg = DxpPackage::from_manifest(TEST_MANIFEST).unwrap();
        assert!(!pkg.is_signed());
        pkg.set_signature(vec![0u8; 64]);
        assert!(pkg.is_signed());
    }
}
