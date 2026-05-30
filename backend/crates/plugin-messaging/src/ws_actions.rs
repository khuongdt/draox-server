use crate::message::{Message, MessageType};
use crate::plugin::PLUGIN_ID;
use crate::store::MessageStore;
use plugin_sdk::traits::WsActionContext;
use serde::Deserialize;
use serde_json::{json, Value};
use server_core::event::ServerEvent;
use server_core::{ClientId, Error, Result};
use std::sync::Arc;

/// Top-level WS action dispatcher for the messaging plugin.
///
/// Routes the per-action payload to the right handler, mutates the store,
/// and emits `ServerEvent::Custom { source: "messaging", name: ..., payload }`
/// events through the bus carried in `ctx.events`. The socket-server
/// forwarder picks those events up and pushes them out to subscribed
/// WS clients.
pub async fn dispatch(
    store: &Arc<MessageStore>,
    ctx: &WsActionContext,
    action: &str,
    payload: Value,
) -> Result<Value> {
    match action {
        "messaging.send_message"      => send_message(store, ctx, payload).await,
        "messaging.subscribe_channel" => subscribe_channel(store, ctx, payload).await,
        "messaging.unsubscribe_channel" => unsubscribe_channel(store, ctx, payload).await,
        "messaging.typing"            => typing(ctx, payload).await,
        other => Err(Error::Plugin {
            plugin_id: PLUGIN_ID.to_string(),
            message:  format!("unknown messaging action: {other}"),
        }),
    }
}

#[derive(Deserialize)]
struct SendMessagePayload {
    channel_id: String,
    text:       String,
    #[serde(default)]
    reply_to_id: Option<String>,
}

async fn send_message(
    store: &Arc<MessageStore>,
    ctx: &WsActionContext,
    payload: Value,
) -> Result<Value> {
    let req: SendMessagePayload = parse(payload)?;
    if req.text.trim().is_empty() {
        return Err(plugin_err("text must not be empty"));
    }
    if store.get_channel(&req.channel_id).is_none() {
        return Err(plugin_err(format!("channel not found: {}", req.channel_id)));
    }
    let from = ClientId::from_str(&ctx.identity.user_id);
    let mut msg = Message::new(MessageType::Channel, from, req.channel_id.clone(), req.text);
    if let Some(reply_to) = req.reply_to_id {
        msg = msg.with_reply_to(reply_to);
    }
    store.store_message(msg.clone());

    let msg_dto = crate::http_api::MessageDto::from(msg);
    let event_payload = json!({ "message": msg_dto });
    ctx.events.publish(ServerEvent::Custom {
        source: "messaging".to_string(),
        name:   "message_sent".to_string(),
        payload: event_payload.clone(),
    });
    Ok(event_payload)
}

#[derive(Deserialize)]
struct ChannelPayload {
    channel_id: String,
}

async fn subscribe_channel(
    store: &Arc<MessageStore>,
    ctx: &WsActionContext,
    payload: Value,
) -> Result<Value> {
    let req: ChannelPayload = parse(payload)?;
    let client = ClientId::from_str(&ctx.identity.user_id);
    store.subscribe_channel(&req.channel_id, &client)?;
    ctx.events.publish(ServerEvent::Custom {
        source: "messaging".to_string(),
        name:   "user_joined_channel".to_string(),
        payload: json!({
            "channel_id": req.channel_id,
            "user_id":    ctx.identity.user_id,
        }),
    });
    Ok(json!({ "ok": true }))
}

async fn unsubscribe_channel(
    store: &Arc<MessageStore>,
    ctx: &WsActionContext,
    payload: Value,
) -> Result<Value> {
    let req: ChannelPayload = parse(payload)?;
    let client = ClientId::from_str(&ctx.identity.user_id);
    store.unsubscribe_channel(&req.channel_id, &client)?;
    ctx.events.publish(ServerEvent::Custom {
        source: "messaging".to_string(),
        name:   "user_left_channel".to_string(),
        payload: json!({
            "channel_id": req.channel_id,
            "user_id":    ctx.identity.user_id,
        }),
    });
    Ok(json!({ "ok": true }))
}

async fn typing(ctx: &WsActionContext, payload: Value) -> Result<Value> {
    let req: ChannelPayload = parse(payload)?;
    ctx.events.publish(ServerEvent::Custom {
        source: "messaging".to_string(),
        name:   "typing_started".to_string(),
        payload: json!({
            "channel_id": req.channel_id,
            "user_id":    ctx.identity.user_id,
            "username":   ctx.identity.user_id,
            "is_typing":  true,
        }),
    });
    Ok(json!({ "ok": true }))
}

// ── helpers ─────────────────────────────────────────────────────────────────

fn parse<T: for<'de> Deserialize<'de>>(value: Value) -> Result<T> {
    serde_json::from_value(value).map_err(|e| plugin_err(format!("invalid payload: {e}")))
}

fn plugin_err(msg: impl Into<String>) -> Error {
    Error::Plugin {
        plugin_id: PLUGIN_ID.to_string(),
        message:   msg.into(),
    }
}
