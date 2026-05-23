pub mod context;
pub mod ipc;
pub mod manifest;
pub mod traits;

pub use context::PluginContext;
pub use ipc::{IpcError, PluginService, ServiceRegistry};
pub use manifest::PluginManifest;
pub use traits::*;
