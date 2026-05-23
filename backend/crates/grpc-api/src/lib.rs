pub mod server;
pub mod state;
pub mod interceptor;
pub mod service;

pub use server::GrpcServer;
pub use state::GrpcState;

// Include generated protobuf/tonic code.
pub mod proto {
    tonic::include_proto!("draox.v1");
}
