pub mod context_builder;
pub mod dependency_graph;
pub mod dir_watcher;
pub mod handles;
pub mod lifecycle;
pub mod loader;
pub mod marketplace;
pub mod marketplace_client;
pub mod marketplace_registry;
pub mod marketplace_types;
pub mod package;
pub mod permissions;
pub mod registry;
pub mod route_registry;
pub mod signature;
pub mod state_persistence;
pub mod update_checker;
pub mod version_resolver;
pub mod ws_dispatcher;

pub use context_builder::ContextBuilder;
pub use dependency_graph::{DependencyError, DependencyGraph};
pub use dir_watcher::{DirWatcher, PluginFileEvent};
pub use loader::PluginLoader;
// Lightweight catalogue registry (MarketplaceEntry) from the existing marketplace module
pub use marketplace::{MarketplaceEntry, MarketplaceRegistry};
// Full-featured in-memory Phase-14B registry aliased to avoid name collision
pub use marketplace_registry::MarketplaceRegistry as FullMarketplaceRegistry;
// Marketplace data types (MarketplaceClient here is the thin config stub)
pub use marketplace_types::{
    MarketplaceClient, MarketplacePlugin, NewReview, PluginAnalytics, PluginCategory,
    PluginDependency, PluginReview, PluginVersion, PublisherInfo, SearchQuery, SearchResult, SortBy,
};
pub use package::DxpPackage;
pub use permissions::{PermissionEnforcer, PluginPermission};
pub use registry::{PluginInfo, PluginRegistry, RestartPolicy};
pub use route_registry::{RouteDefinition, RouteRegistry};
pub use signature::SignatureVerifier;
pub use state_persistence::{PersistedPluginState, StatePersistence};
// Phase 14B/14C: full registry client, update checker, version resolver
pub use marketplace_client::RegistryClient;
pub use update_checker::UpdateChecker;
pub use version_resolver::VersionResolver;
pub use ws_dispatcher::PluginWsDispatcher;
