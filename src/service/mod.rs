pub mod handlers;
pub mod sync_service;

// Generated protobuf types — shared across all handlers.
pub mod sync {
    tonic::include_proto!("backstep.sync.v1");
}

pub use sync_service::{AppState, SyncServiceImpl};
