use connection_manager::SessionManager;
use plugin_host::PluginRegistry;
use server_core::event::EventBus;
use std::sync::Arc;

#[derive(Clone)]
pub struct GrpcState {
    pub session_manager: Arc<SessionManager>,
    pub event_bus:       Arc<EventBus>,
    pub plugin_registry: Arc<PluginRegistry>,
}
