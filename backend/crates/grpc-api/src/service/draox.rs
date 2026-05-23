use crate::proto::draox_service_server::DraoxService;
use crate::proto::{DraoxEvent, DraoxRequest, DraoxResponse, SubscribeRequest};
use crate::state::GrpcState;
use futures_util::StreamExt;
use server_core::event::ServerEvent;
use server_core::SessionId;
use std::pin::Pin;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::Stream;
use tonic::{Request, Response, Status};

pub struct DraoxServiceImpl {
    state: GrpcState,
}

impl DraoxServiceImpl {
    pub fn new(state: GrpcState) -> Self {
        Self { state }
    }

    fn resolve_session(&self, session_id_str: &str) -> Result<SessionId, Status> {
        let sid = SessionId::from_str(session_id_str);
        if self.state.session_manager.get_session(&sid).is_none() {
            return Err(Status::unauthenticated("invalid or expired session"));
        }
        Ok(sid)
    }
}

#[tonic::async_trait]
impl DraoxService for DraoxServiceImpl {
    async fn send(
        &self,
        request: Request<DraoxRequest>,
    ) -> Result<Response<DraoxResponse>, Status> {
        let req = request.into_inner();

        let payload: serde_json::Value = if req.payload.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_slice(&req.payload)
                .map_err(|_| Status::invalid_argument("invalid payload JSON"))?
        };

        self.state.event_bus.publish(ServerEvent::Custom {
            source:  "grpc".to_string(),
            name:    req.action.clone(),
            payload: payload.clone(),
        });

        let resp_data = serde_json::json!({ "action": req.action, "received": true });
        let data_bytes = serde_json::to_vec(&resp_data).unwrap_or_default();

        Ok(Response::new(DraoxResponse {
            id:      req.id,
            success: true,
            data:    data_bytes,
            error:   String::new(),
        }))
    }

    type SubscribeStream = Pin<Box<dyn Stream<Item = Result<DraoxEvent, Status>> + Send>>;

    async fn subscribe(
        &self,
        request: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let req = request.into_inner();
        self.resolve_session(&req.session_id)?;

        let categories = req.categories;
        let rx = self.state.event_bus.subscribe_all();

        let stream = BroadcastStream::new(rx).filter_map(move |result| {
            let cats = categories.clone();
            async move {
                let event = result.ok()?;
                let (category, name, data) = match event.as_ref() {
                    ServerEvent::Custom { source, name, payload } => (
                        source.clone(),
                        name.clone(),
                        serde_json::to_vec(payload).unwrap_or_default(),
                    ),
                    _ => return None,
                };

                if !cats.is_empty() && !cats.contains(&category) {
                    return None;
                }

                Some(Ok(DraoxEvent {
                    category,
                    name,
                    data,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                }))
            }
        });

        Ok(Response::new(Box::pin(stream)))
    }
}
