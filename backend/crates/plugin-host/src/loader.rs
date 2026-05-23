use crate::package::DxpPackage;
use crate::signature::SignatureVerifier;
use dashmap::DashMap;
use server_core::{Error, PluginId, Result};
use tracing::info;

/// Manages installed plugin packages.
///
/// Scans a plugins directory, installs/uninstalls packages,
/// and verifies signatures.
pub struct PluginLoader {
    installed: DashMap<PluginId, DxpPackage>,
    verifier: SignatureVerifier,
    require_signature: bool,
}

impl PluginLoader {
    pub fn new(require_signature: bool) -> Self {
        Self {
            installed: DashMap::new(),
            verifier: SignatureVerifier::new(),
            require_signature,
        }
    }

    /// Get the signature verifier for configuring trusted keys.
    pub fn verifier_mut(&mut self) -> &mut SignatureVerifier {
        &mut self.verifier
    }

    /// Install a plugin package.
    pub fn install(&self, package: DxpPackage) -> Result<PluginId> {
        let plugin_id = package.plugin_id();

        // Check if already installed
        if self.installed.contains_key(&plugin_id) {
            return Err(Error::Plugin {
                plugin_id: plugin_id.to_string(),
                message: "plugin already installed".to_string(),
            });
        }

        // Validate package structure
        package.validate()?;

        // Verify signature if required
        if self.require_signature {
            if !package.is_signed() {
                return Err(Error::Plugin {
                    plugin_id: plugin_id.to_string(),
                    message: "package signature required but not provided".to_string(),
                });
            }

            // In production, verify the signature against manifest hash
            if let Some(sig) = &package.signature {
                let manifest_bytes = serde_json::to_vec(&package.manifest)
                    .map_err(|e| Error::Plugin {
                        plugin_id: plugin_id.to_string(),
                        message: format!("failed to serialize manifest: {e}"),
                    })?;

                match self.verifier.verify(&manifest_bytes, sig) {
                    Ok(true) => {
                        info!(plugin_id = %plugin_id, "package signature verified");
                    }
                    Ok(false) => {
                        return Err(Error::Plugin {
                            plugin_id: plugin_id.to_string(),
                            message: "package signature verification failed".to_string(),
                        });
                    }
                    Err(e) => {
                        return Err(Error::Plugin {
                            plugin_id: plugin_id.to_string(),
                            message: format!("signature verification error: {e}"),
                        });
                    }
                }
            }
        }

        info!(
            plugin_id = %plugin_id,
            name = %package.manifest.name,
            version = %package.manifest.version,
            "plugin package installed"
        );

        self.installed.insert(plugin_id.clone(), package);
        Ok(plugin_id)
    }

    /// Uninstall a plugin package.
    pub fn uninstall(&self, id: &PluginId) -> Result<()> {
        if self.installed.remove(id).is_some() {
            info!(plugin_id = %id, "plugin package uninstalled");
            Ok(())
        } else {
            Err(Error::PluginNotFound(id.to_string()))
        }
    }

    /// Get an installed package.
    pub fn get_package(&self, id: &PluginId) -> Option<DxpPackage> {
        self.installed.get(id).map(|r| r.value().clone())
    }

    /// List all installed packages.
    pub fn list_installed(&self) -> Vec<DxpPackage> {
        self.installed.iter().map(|r| r.value().clone()).collect()
    }

    /// Number of installed packages.
    pub fn installed_count(&self) -> usize {
        self.installed.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_package() -> DxpPackage {
        DxpPackage::from_manifest(
            r#"
id = "io.draox.test"
name = "Test"
version = "1.0.0"
author = "Test"
type = "builtin"
"#,
        )
        .unwrap()
    }

    #[test]
    fn test_install_package() {
        let loader = PluginLoader::new(false);
        let pkg = test_package();
        let id = loader.install(pkg).unwrap();
        assert_eq!(loader.installed_count(), 1);
        assert!(loader.get_package(&id).is_some());
    }

    #[test]
    fn test_duplicate_install_fails() {
        let loader = PluginLoader::new(false);
        loader.install(test_package()).unwrap();
        assert!(loader.install(test_package()).is_err());
    }

    #[test]
    fn test_uninstall() {
        let loader = PluginLoader::new(false);
        let id = loader.install(test_package()).unwrap();
        loader.uninstall(&id).unwrap();
        assert_eq!(loader.installed_count(), 0);
    }

    #[test]
    fn test_uninstall_not_found() {
        let loader = PluginLoader::new(false);
        let id = PluginId::from_str("io.draox.nonexistent");
        assert!(loader.uninstall(&id).is_err());
    }

    #[test]
    fn test_require_signature_blocks_unsigned() {
        let loader = PluginLoader::new(true);
        let pkg = test_package();
        assert!(!pkg.is_signed());
        let result = loader.install(pkg);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_installed() {
        let loader = PluginLoader::new(false);
        loader.install(test_package()).unwrap();

        let pkg2 = DxpPackage::from_manifest(
            r#"
id = "io.draox.other"
name = "Other"
version = "2.0.0"
author = "Test"
type = "builtin"
"#,
        )
        .unwrap();
        loader.install(pkg2).unwrap();

        let list = loader.list_installed();
        assert_eq!(list.len(), 2);
    }
}
