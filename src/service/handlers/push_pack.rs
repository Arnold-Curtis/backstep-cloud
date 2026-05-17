use tonic::{Request, Response, Status, Streaming};

use crate::auth::authenticate;
use crate::db::packs;
use crate::db::validation;
use crate::logging;
use crate::service::sync::*;
use crate::service::sync_service::AppState;
use crate::storage;

pub async fn handle(
    request: Request<Streaming<PackPushRequest>>,
    state: &AppState,
) -> Result<Response<PackPushResponse>, Status> {
    let ctx = authenticate(request.metadata(), &state.pool)
        .await
        .map_err(Status::from)?;

    let _permit = state.rate_limiter.acquire(ctx.account_id).await;

    let mut stream = request.into_inner();
    let mut buffer = Vec::new();
    let mut metadata: Option<PackPushRequest> = None;

    while let Some(msg) = stream.message().await.map_err(Status::from)? {
        if metadata.is_none() {
            validation::validate_device_id(&msg.device_id).map_err(Status::from)?;
            validation::validate_pack_size(msg.total_bytes, state.max_pack_bytes)
                .map_err(Status::from)?;

            buffer.reserve(msg.total_bytes as usize);
            metadata = Some(msg);
        } else {
            buffer.extend_from_slice(&msg.chunk_data);
        }
    }

    let meta = metadata.ok_or_else(|| Status::invalid_argument("empty stream"))?;

    if buffer.len() as u64 != meta.total_bytes {
        return Err(Status::data_loss(format!(
            "truncated stream: expected {} bytes, received {}",
            meta.total_bytes,
            buffer.len()
        )));
    }

    let etag = storage::upload_pack(
        &state.r2_client,
        &state.r2_bucket,
        ctx.account_id,
        meta.pack_id as i64,
        buffer,
    )
    .await
    .map_err(Status::from)?;

    packs::update_pack_ready(&state.pool, ctx.account_id, meta.pack_id as i64, &etag)
        .await
        .map_err(Status::from)?;

    let server_clock = crate::db::state::get_clock(&state.pool, ctx.account_id)
        .await
        .map_err(Status::from)?;

    logging::audit_mutation(
        ctx.account_id,
        &meta.device_id,
        "push_pack",
        "pack",
        meta.pack_id as i64,
        server_clock,
    );

    Ok(Response::new(PackPushResponse {
        pack_id: meta.pack_id,
        server_clock: server_clock as u64,
        status: "ok".into(),
    }))
}
