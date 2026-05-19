use tonic::{Request, Response, Status};

use crate::auth::authenticate;
use crate::db::{events, state, validation};
use crate::logging;
use crate::service::sync::*;
use crate::service::sync_service::AppState;

pub async fn handle(
    request: Request<MetadataPullRequest>,
    state: &AppState,
) -> Result<Response<MetadataPullResponse>, Status> {
    let ctx = authenticate(request.metadata(), &state.pool)
        .await
        .map_err(Status::from)?;

    let req = request.into_inner();

    state.rate_limiter.acquire(ctx.account_id).await;

    validation::validate_device_id(&req.device_id).map_err(Status::from)?;
    validation::validate_since_clock(req.since_clock).map_err(Status::from)?;

    let limit = req
        .max_operations
        .min(state.max_pull_operations)
        .max(1) as i64;

    let (records, has_more) =
        events::query_since_clock(&state.pool, ctx.account_id, req.since_clock as i64, limit)
            .await
            .map_err(Status::from)?;

    let server_clock = state::get_clock(&state.pool, ctx.account_id)
        .await
        .map_err(Status::from)?;

    let operations: Vec<RemoteOperation> = records
        .into_iter()
        .map(|r| RemoteOperation {
            origin_device_id: r.origin_device_id,
            server_clock: r.server_clock as u64,
            entity_type: str_to_entity_type(&r.entity_type),
            operation: str_to_sync_operation(&r.operation),
            entity_id: r.entity_id as u64,
            entity_sub_id: r.entity_sub_id as u64,
            timestamp: r.event_timestamp,
            encrypted_metadata: r.encrypted_metadata,
        })
        .collect();

    logging::audit_access(ctx.account_id, &req.device_id, "pull_metadata");

    Ok(Response::new(MetadataPullResponse {
        operations,
        server_clock: server_clock as u64,
        has_more,
    }))
}

fn str_to_entity_type(s: &str) -> i32 {
    match s {
        "version" => EntityType::Version as i32,
        "chunk" => EntityType::Chunk as i32,
        "tombstone" => EntityType::Tombstone as i32,
        _ => EntityType::Unspecified as i32,
    }
}

fn str_to_sync_operation(s: &str) -> i32 {
    match s {
        "create" => SyncOperation::Create as i32,
        "delete" => SyncOperation::Delete as i32,
        _ => SyncOperation::Unspecified as i32,
    }
}
