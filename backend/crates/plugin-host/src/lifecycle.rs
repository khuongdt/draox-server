use plugin_sdk::traits::PluginState;
use server_core::{Error, Result};

/// Validate a plugin state transition.
///
/// Valid transitions:
/// - Installed → ActiveEnabled (activate)
/// - ActiveEnabled → ActiveDisabled (disable)
/// - ActiveDisabled → ActiveEnabled (enable)
/// - ActiveEnabled → Installed (deactivate)
/// - ActiveDisabled → Installed (deactivate)
/// - Any → Uninstalled (uninstall)
pub fn validate_transition(from: PluginState, to: PluginState) -> Result<()> {
    let valid = matches!(
        (from, to),
        // Activation
        (PluginState::Installed, PluginState::ActiveEnabled)
        // Enable / Disable
        | (PluginState::ActiveEnabled, PluginState::ActiveDisabled)
        | (PluginState::ActiveDisabled, PluginState::ActiveEnabled)
        // Deactivation
        | (PluginState::ActiveEnabled, PluginState::Installed)
        | (PluginState::ActiveDisabled, PluginState::Installed)
        // Uninstall from any state
        | (PluginState::Installed, PluginState::Uninstalled)
        | (PluginState::ActiveEnabled, PluginState::Uninstalled)
        | (PluginState::ActiveDisabled, PluginState::Uninstalled)
    );

    if valid {
        Ok(())
    } else {
        Err(Error::Plugin {
            plugin_id: String::new(),
            message: format!("invalid state transition: {from:?} → {to:?}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        // Activate
        assert!(validate_transition(PluginState::Installed, PluginState::ActiveEnabled).is_ok());
        // Disable
        assert!(
            validate_transition(PluginState::ActiveEnabled, PluginState::ActiveDisabled).is_ok()
        );
        // Enable
        assert!(
            validate_transition(PluginState::ActiveDisabled, PluginState::ActiveEnabled).is_ok()
        );
        // Deactivate from enabled
        assert!(validate_transition(PluginState::ActiveEnabled, PluginState::Installed).is_ok());
        // Deactivate from disabled
        assert!(validate_transition(PluginState::ActiveDisabled, PluginState::Installed).is_ok());
        // Uninstall
        assert!(validate_transition(PluginState::Installed, PluginState::Uninstalled).is_ok());
        assert!(validate_transition(PluginState::ActiveEnabled, PluginState::Uninstalled).is_ok());
    }

    #[test]
    fn test_invalid_transitions() {
        // Can't go from Installed to Disabled directly
        assert!(
            validate_transition(PluginState::Installed, PluginState::ActiveDisabled).is_err()
        );
        // Can't re-install from Uninstalled
        assert!(validate_transition(PluginState::Uninstalled, PluginState::Installed).is_err());
        // Can't activate from Uninstalled
        assert!(
            validate_transition(PluginState::Uninstalled, PluginState::ActiveEnabled).is_err()
        );
        // Can't go from ActiveEnabled to ActiveEnabled (no-op is invalid)
        assert!(
            validate_transition(PluginState::ActiveEnabled, PluginState::ActiveEnabled).is_err()
        );
    }
}
