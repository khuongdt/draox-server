//! Procedural macros for the Draox Server plugin system.
//!
//! # `#[draox_plugin]`
//!
//! Attribute macro that marks a struct as a Draox plugin and generates a
//! `create_<snake_case_name>()` factory function that returns a
//! `Box<dyn plugin_sdk::Plugin>`.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use draox_macros::draox_plugin;
//!
//! #[draox_plugin]
//! pub struct MyPlugin {
//!     name: String,
//! }
//! ```
//!
//! The above expands to (approximately):
//!
//! ```rust,ignore
//! pub struct MyPlugin { name: String }
//!
//! /// Auto-generated factory function for `MyPlugin`.
//! /// Returns a heap-allocated plugin instance ready for registration.
//! pub fn create_my_plugin() -> Box<dyn ::plugin_sdk::Plugin> {
//!     Box::new(MyPlugin::default())
//! }
//! ```

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Ident};

// ─────────────────────────────────────────────────────────────────────────────
// Helper — convert CamelCase to snake_case
// ─────────────────────────────────────────────────────────────────────────────

/// Convert a CamelCase identifier string to snake_case.
/// Example: "MyPlugin" → "my_plugin", "HTTPServer" → "h_t_t_p_server".
fn camel_to_snake(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 8);
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && i != 0 {
            out.push('_');
        }
        out.push(ch.to_ascii_lowercase());
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// #[draox_plugin] attribute macro
// ─────────────────────────────────────────────────────────────────────────────

/// Attribute macro that marks a struct as a Draox plugin.
///
/// Generates a `create_<snake_case_name>()` factory function that constructs
/// the plugin via `Default::default()` and boxes it as `Box<dyn Plugin>`.
///
/// # Requirements
///
/// The annotated struct must implement both `plugin_sdk::Plugin` and
/// `Default` (so the factory function can instantiate it without arguments).
///
/// # Example
///
/// ```rust,ignore
/// use draox_macros::draox_plugin;
///
/// #[draox_plugin]
/// pub struct GreeterPlugin;
///
/// // Generated:
/// // pub fn create_greeter_plugin() -> Box<dyn plugin_sdk::Plugin> {
/// //     Box::new(GreeterPlugin::default())
/// // }
/// ```
#[proc_macro_attribute]
pub fn draox_plugin(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Reject any attribute arguments — the macro takes no parameters.
    if !attr.is_empty() {
        return syn::Error::new(
            Span::call_site(),
            "#[draox_plugin] does not accept arguments",
        )
        .to_compile_error()
        .into();
    }

    // Parse the annotated item as a struct/enum/union definition.
    let input = parse_macro_input!(item as DeriveInput);
    let struct_name = &input.ident;

    // Build the factory function name: create_<snake_case>.
    let snake = camel_to_snake(&struct_name.to_string());
    let factory_fn = Ident::new(&format!("create_{snake}"), struct_name.span());

    // Re-emit the original item unchanged, then append the factory function.
    let expanded = quote! {
        // Original struct definition (unchanged).
        #input

        /// Auto-generated factory function for [`#struct_name`].
        ///
        /// Returns a heap-allocated, type-erased plugin instance ready for
        /// registration with the Draox plugin host.
        ///
        /// The plugin is constructed via [`Default::default()`]; ensure the
        /// struct implements `Default`.
        pub fn #factory_fn() -> ::std::boxed::Box<dyn ::plugin_sdk::Plugin> {
            ::std::boxed::Box::new(#struct_name::default())
        }
    };

    expanded.into()
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests (non-proc-macro logic only)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::camel_to_snake;

    #[test]
    fn test_camel_to_snake_simple() {
        assert_eq!(camel_to_snake("MyPlugin"), "my_plugin");
    }

    #[test]
    fn test_camel_to_snake_single_word() {
        assert_eq!(camel_to_snake("Plugin"), "plugin");
    }

    #[test]
    fn test_camel_to_snake_already_lower() {
        assert_eq!(camel_to_snake("plugin"), "plugin");
    }

    #[test]
    fn test_camel_to_snake_multiple_words() {
        assert_eq!(camel_to_snake("GreeterPlugin"), "greeter_plugin");
    }

    #[test]
    fn test_camel_to_snake_empty() {
        assert_eq!(camel_to_snake(""), "");
    }

    #[test]
    fn test_factory_fn_name_format() {
        // Verify the naming convention used inside the macro.
        let struct_name = "ClanPlugin";
        let snake = camel_to_snake(struct_name);
        let factory = format!("create_{snake}");
        assert_eq!(factory, "create_clan_plugin");
    }
}
