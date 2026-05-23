use crate::proto::auth_service_server::AuthService;
use crate::proto::{AuthRequest, AuthResponse};
use crate::state::GrpcState;
use server_core::{ClientId, SessionId};
use tonic::{Request, Response, Status};

pub struct AuthServiceImpl {
    state: GrpcState,
}

impl AuthServiceImpl {
    pub fn new(state: GrpcState) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl AuthService for AuthServiceImpl {
    async fn authenticate(
        &self,
        request: Request<AuthRequest>,
    ) -> Result<Response<AuthResponse>, Status> {
        let req = request.into_inner();

        if req.user_id.is_empty() || req.token.is_empty() {
            return Ok(Response::new(AuthResponse {
                success:    false,
                session_id: String::new(),
                error:      "user_id and token are required".to_string(),
            }));
        }

        // Create a session keyed by the user_id as client identity.
        let client_id = ClientId::from_str(&req.user_id);
        let session_id: SessionId = self.state.session_manager.create_session(client_id);

        Ok(Response::new(AuthResponse {
            success:    true,
            session_id: session_id.to_string(),
            error:      String::new(),
        }))
    }
}
