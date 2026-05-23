use crate::proto::messaging_service_server::MessagingService;
use crate::proto::{
    AddReactionRequest, DeleteMessageRequest, EditMessageRequest, HistoryRequest,
    HistoryResponse, Message, MessageEvent, MutationResponse, SendMessageRequest,
    SendMessageResponse, SubscribeChannelRequest,
};
use crate::state::GrpcState;
use futures_util::StreamExt;
use server_core::event::ServerEvent;
use server_core::SessionId;
use std::pin::Pin;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::Stream;
use tonic::{Request, Response, Status};

pub struct MessagingServiceImpl {
    state: GrpcState,
}

impl MessagingServiceImpl {
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

    fn dispatch(&self, action: &str, session_id: &str, payload: serde_json::Value) {
        self.state.event_bus.publish(ServerEvent::Custom {
            source:  "grpc".to_string(),
            name:    format!("msg.{action}"),
            payload: serde_json::json!({
                "session_id": session_id,
                "action":     action,
                "data":       payload,
            }),
        });
    }
}

#[tonic::async_trait]
impl MessagingService for MessagingServiceImpl {
    async fn send_message(
        &self,
        request: Request<SendMessageRequest>,
    ) -> Result<Response<SendMessageResponse>, Status> {
        let req = request.into_inner();
        self.resolve_session(&req.session_id)?;

        let msg_id = uuid::Uuid::new_v4().to_string();
        self.dispatch(
            "send",
            &req.session_id,
            serde_json::json!({
                "channel":  req.channel,
                "text":     req.text,
                "reply_to": req.reply_to,
                "id":       msg_id.clone(),
            }),
        );

        Ok(Response::new(SendMessageResponse {
            success:    true,
            message_id: msg_id,
            error:      String::new(),
        }))
    }

    async fn get_history(
        &self,
        request: Request<HistoryRequest>,
    ) -> Result<Response<HistoryResponse>, Status> {
        let req = request.into_inner();
        self.resolve_session(&req.session_id)?;

        self.dispatch(
            "history",
            &req.session_id,
            serde_json::json!({
                "channel":   req.channel,
                "limit":     req.limit,
                "before_id": req.before_id,
            }),
        );

        // Full response requires a request-reply channel to the plugin.
        // Returning empty list here — callers should use SubscribeChannel for live updates.
        Ok(Response::new(HistoryResponse { messages: vec![], error: String::new() }))
    }

    async fn delete_message(
        &self,
        request: Request<DeleteMessageRequest>,
    ) -> Result<Response<MutationResponse>, Status> {
        let req = request.into_inner();
        self.resolve_session(&req.session_id)?;
        self.dispatch("delete", &req.session_id, serde_json::json!({ "message_id": req.message_id }));
        Ok(Response::new(MutationResponse { success: true, error: String::new() }))
    }

    async fn edit_message(
        &self,
        request: Request<EditMessageRequest>,
    ) -> Result<Response<MutationResponse>, Status> {
        let req = request.into_inner();
        self.resolve_session(&req.session_id)?;
        self.dispatch(
            "edit",
            &req.session_id,
            serde_json::json!({ "message_id": req.message_id, "new_text": req.new_text }),
        );
        Ok(Response::new(MutationResponse { success: true, error: String::new() }))
    }

    async fn add_reaction(
        &self,
        request: Request<AddReactionRequest>,
    ) -> Result<Response<MutationResponse>, Status> {
        let req = request.into_inner();
        self.resolve_session(&req.session_id)?;
        self.dispatch(
            "react",
            &req.session_id,
            serde_json::json!({ "message_id": req.message_id, "emoji": req.emoji }),
        );
        Ok(Response::new(MutationResponse { success: true, error: String::new() }))
    }

    type SubscribeChannelStream =
        Pin<Box<dyn Stream<Item = Result<MessageEvent, Status>> + Send>>;

    async fn subscribe_channel(
        &self,
        request: Request<SubscribeChannelRequest>,
    ) -> Result<Response<Self::SubscribeChannelStream>, Status> {
        let req = request.into_inner();
        self.resolve_session(&req.session_id)?;

        let channel = req.channel.clone();
        let rx = self.state.event_bus.subscribe_all();

        let stream = BroadcastStream::new(rx).filter_map(move |result| {
            let ch = channel.clone();
            async move {
                let event = result.ok()?;
                let (event_type, msg_data) = match event.as_ref() {
                    ServerEvent::Custom { name, payload, .. } if name.starts_with("msg.") => {
                        let payload_ch = payload.get("data")
                            .and_then(|d| d.get("channel"))
                            .and_then(|v| v.as_str())?;
                        if payload_ch != ch {
                            return None;
                        }
                        let ev_type = name.trim_start_matches("msg.").to_string();
                        let data = payload.get("data").cloned().unwrap_or(serde_json::Value::Null);
                        (ev_type, data)
                    }
                    _ => return None,
                };

                let message = Message {
                    id:        msg_data["id"].as_str().unwrap_or_default().to_string(),
                    channel:   msg_data["channel"].as_str().unwrap_or_default().to_string(),
                    sender_id: msg_data["sender_id"].as_str().unwrap_or_default().to_string(),
                    text:      msg_data["text"].as_str().unwrap_or_default().to_string(),
                    timestamp: msg_data["timestamp"].as_str().unwrap_or_default().to_string(),
                };

                Some(Ok(MessageEvent {
                    event_type,
                    message: Some(message),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                }))
            }
        });

        Ok(Response::new(Box::pin(stream)))
    }
}
