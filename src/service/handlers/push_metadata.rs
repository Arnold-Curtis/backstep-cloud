use tonic::{Request, Response, Status};

use crate::auth::authenticate;
use crate::db::{events, state, validation};
use crate::logging;
use crate::service::sync::*;
use crate::service::sync_service::AppState;

pub async fn handle(
    request: Request<MetadataPushRequest>,
    state: &AppState,
) -> Result<Response<MetadataPushResponse>, Status> {
    let ctx = authenticate(request.metadata(), &state.pool)
        .await
        .map_err(Status::from)?;

    let req = request.into_inner();

    state.rate_limiter.acquire(ctx.account_id).await;

    let entity_type = EntityType::try_from(req.entity_type)
        .map_err(|_| Status::invalid_argument("invalid entity_type"))?;
    let operation = SyncOperation::try_from(req.operation)
        .map_err(|_| Status::invalid_argument("invalid operation"))?;

    validation::validate_device_id(&req.device_id).map_err(Status::from)?;
    validation::validate_entity_type(entity_type.as_str_name()).map_err(Status::from)?;
    validation::validate_operation(operation.as_str_name()).map_err(Status::from)?;

    let record = events::EventRecord {
        origin_device_id: req.device_id.clone(),
        server_clock: 0,
        entity_type: entity_type.as_str_name().to_string(),
        operation: operation.as_str_name().to_string(),
        entity_id: req.entity_id as i64,
        entity_sub_id: req.entity_sub_id as i64,
        event_timestamp: req.timestamp.clone(),
        encrypted_metadata: req.encrypted_metadata.clone(),
    };

    // Atomic clock increment + event insert in one transaction.
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| Status::internal(e.to_string()))?;
    let new_clock = state::increment_clock(&mut tx, ctx.account_id, req.lamport_clock as i64)
        .await
        .map_err(|e| Status::internal(e.to_string()))?;

    let mut record_with_clock = record;
    record_with_clock.server_clock = new_clock;

    events::insert_event(&mut *tx, ctx.account_id, &record_with_clock)
        .await
        .map_err(|e| Status::internal(e.to_string()))?;
    tx.commit()
        .await
        .map_err(|e| Status::internal(e.to_string()))?;

    logging::audit_mutation(
        ctx.account_id,
        &req.device_id,
        "push_metadata",
        entity_type.as_str_name(),
        req.entity_id as i64,
        new_clock,
    );

    Ok(Response::new(MetadataPushResponse {
        accepted_count: 1,
        server_clock: new_clock as u64,
        status: "ok".into(),
    }))
}
