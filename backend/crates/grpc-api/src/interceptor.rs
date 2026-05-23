use tonic::{Request, Status};

// Extracts and validates session-id from gRPC metadata.
// Usage: wrap service with .layer(tonic::service::interceptor(auth_interceptor))
pub fn auth_interceptor(mut req: Request<()>) -> Result<Request<()>, Status> {
    let meta = req.metadata();

    let session_id = meta
        .get("x-session-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);

    if let Some(sid) = session_id {
        req.extensions_mut().insert(SessionIdExt(sid));
    }

    Ok(req)
}

// Extension type to carry session id through request extensions.
#[derive(Clone)]
pub struct SessionIdExt(pub String);
