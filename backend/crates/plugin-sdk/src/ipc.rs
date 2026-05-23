use std::sync::Arc;
use async_trait::async_trait;
use dashmap::DashMap;
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IpcError {
    #[error("service not found: {0}")]
    ServiceNotFound(String),
    #[error("method not found: {method} on {service}")]
    MethodNotFound { service: String, method: String },
    #[error("rpc call failed on {service}::{method}: {reason}")]
    CallFailed {
        service: String,
        method: String,
        reason: String,
    },
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Typed contract a plugin exposes over the IPC bus.
/// Implement this trait in a plugin crate and register it with [`ServiceRegistry`].
#[async_trait]
pub trait PluginService: Send + Sync + 'static {
    /// Stable service name used as the routing key, e.g. `"clans"`, `"identity"`.
    fn service_name(&self) -> &'static str;

    /// Handle an RPC call. The `method` string selects the operation; `payload`
    /// is free-form JSON so that callers need no direct type dependency on this crate.
    async fn call(&self, method: &str, payload: Value) -> Result<Value, IpcError>;
}

/// Central registry shared via [`PluginContext`].
/// Plugins call [`ServiceRegistry::register`] during activation and
/// [`ServiceRegistry::call`] to invoke another plugin's service.
#[derive(Clone, Default)]
pub struct ServiceRegistry {
    services: Arc<DashMap<String, Arc<dyn PluginService>>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, service: Arc<dyn PluginService>) {
        let name = service.service_name().to_string();
        tracing::debug!(service = %name, "IPC service registered");
        self.services.insert(name, service);
    }

    pub fn unregister(&self, name: &str) {
        self.services.remove(name);
        tracing::debug!(service = %name, "IPC service unregistered");
    }

    /// Invoke `method` on the named service, returning the JSON response.
    pub async fn call(
        &self,
        service: &str,
        method: &str,
        payload: Value,
    ) -> Result<Value, IpcError> {
        let svc = self
            .services
            .get(service)
            .ok_or_else(|| IpcError::ServiceNotFound(service.to_string()))?
            .clone();
        svc.call(method, payload).await
    }

    /// Typed convenience: serializes `req` and deserializes the response.
    pub async fn call_typed<Req, Res>(
        &self,
        service: &str,
        method: &str,
        req: &Req,
    ) -> Result<Res, IpcError>
    where
        Req: serde::Serialize,
        Res: serde::de::DeserializeOwned,
    {
        let payload = serde_json::to_value(req)?;
        let response = self.call(service, method, payload).await?;
        Ok(serde_json::from_value(response)?)
    }

    pub fn is_registered(&self, service: &str) -> bool {
        self.services.contains_key(service)
    }

    pub fn list_services(&self) -> Vec<String> {
        let mut names: Vec<_> = self.services.iter().map(|e| e.key().clone()).collect();
        names.sort();
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    struct EchoService;

    #[async_trait]
    impl PluginService for EchoService {
        fn service_name(&self) -> &'static str {
            "echo"
        }
        async fn call(&self, method: &str, payload: Value) -> Result<Value, IpcError> {
            match method {
                "echo" => Ok(payload),
                _ => Err(IpcError::MethodNotFound {
                    service: "echo".into(),
                    method: method.to_string(),
                }),
            }
        }
    }

    #[tokio::test]
    async fn test_register_and_call() {
        let registry = ServiceRegistry::new();
        registry.register(Arc::new(EchoService));
        assert!(registry.is_registered("echo"));
        let result = registry
            .call("echo", "echo", serde_json::json!({"x": 42}))
            .await
            .unwrap();
        assert_eq!(result["x"], 42);
    }

    #[tokio::test]
    async fn test_service_not_found() {
        let registry = ServiceRegistry::new();
        let err = registry
            .call("missing", "noop", Value::Null)
            .await
            .unwrap_err();
        assert!(matches!(err, IpcError::ServiceNotFound(_)));
    }

    #[tokio::test]
    async fn test_typed_call() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct Msg {
            text: String,
        }
        let registry = ServiceRegistry::new();
        registry.register(Arc::new(EchoService));
        let req = Msg { text: "hello".into() };
        let res: Msg = registry
            .call_typed("echo", "echo", &req)
            .await
            .unwrap();
        assert_eq!(res, req);
    }
}
