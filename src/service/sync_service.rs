use std::pin::Pin;

use sqlx::PgPool;
use tokio_stream::Stream;
use tonic::{Request, Response, Status, Streaming};

use crate::auth::rate_limiter::RateLimiter;
use crate::service::handlers;
use crate::service::sync::sync_service_server::SyncService;
use crate::service::sync::*;

/// Shared application state available to all handlers.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub r2_client: aws_sdk_s3::Client,
    pub r2_bucket: String,
    pub rate_limiter: RateLimiter,
    pub max_pack_bytes: u64,
    pub max_pull_operations: u32,
}

#[derive(Clone)]
pub struct SyncServiceImpl {
    state: AppState,
}

impl SyncServiceImpl {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl SyncService for SyncServiceImpl {
    async fn handshake(
        &self,
        request: Request<SyncHandshakeRequest>,
    ) -> Result<Response<SyncHandshakeResponse>, Status> {
        handlers::handshake::handle(request, &self.state).await
    }

    async fn push_metadata(
        &self,
        request: Request<MetadataPushRequest>,
    ) -> Result<Response<MetadataPushResponse>, Status> {
        handlers::push_metadata::handle(request, &self.state).await
    }

    async fn pull_metadata(
        &self,
        request: Request<MetadataPullRequest>,
    ) -> Result<Response<MetadataPullResponse>, Status> {
        handlers::pull_metadata::handle(request, &self.state).await
    }

    /// Client-streaming: receives PackPushRequest stream, returns single PackPushResponse.
    async fn push_pack(
        &self,
        request: Request<Streaming<PackPushRequest>>,
    ) -> Result<Response<PackPushResponse>, Status> {
        handlers::push_pack::handle(request, &self.state).await
    }

    type PullPackStream = Pin<Box<dyn Stream<Item = Result<PackPullResponse, Status>> + Send>>;

    async fn pull_pack(
        &self,
        request: Request<PackPullRequest>,
    ) -> Result<Response<Self::PullPackStream>, Status> {
        handlers::pull_pack::handle(request, &self.state).await
    }
}
