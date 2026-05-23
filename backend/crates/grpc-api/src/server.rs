use crate::interceptor::auth_interceptor;
use crate::proto::auth_service_server::AuthServiceServer;
use crate::proto::draox_service_server::DraoxServiceServer;
use crate::proto::messaging_service_server::MessagingServiceServer;
use crate::service::auth::AuthServiceImpl;
use crate::service::draox::DraoxServiceImpl;
use crate::service::messaging::MessagingServiceImpl;
use crate::state::GrpcState;
use server_core::ShutdownReceiver;
use std::net::SocketAddr;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server;
use tracing::info;

pub struct GrpcServer;

impl GrpcServer {
    pub async fn start(
        addr: SocketAddr,
        state: GrpcState,
        mut shutdown: ShutdownReceiver,
    ) -> server_core::Result<SocketAddr> {
        let auth_svc      = AuthServiceServer::new(AuthServiceImpl::new(state.clone()));
        let draox_svc     = DraoxServiceServer::new(DraoxServiceImpl::new(state.clone()));
        let messaging_svc = MessagingServiceServer::new(MessagingServiceImpl::new(state.clone()));

        let listener = tokio::net::TcpListener::bind(addr).await?;
        let bound    = listener.local_addr()?;

        info!("gRPC server bound to {bound}");

        let router = Server::builder()
            .layer(tonic::service::interceptor(auth_interceptor))
            .add_service(auth_svc)
            .add_service(draox_svc)
            .add_service(messaging_svc);

        tokio::spawn(async move {
            let incoming = TcpListenerStream::new(listener);
            if let Err(e) = router
                .serve_with_incoming_shutdown(incoming, async move { shutdown.recv().await })
                .await
            {
                tracing::error!("gRPC server error: {e}");
            }
        });

        Ok(bound)
    }
}
