use tonic::{Request, Response, Status};

use crate::auth::authenticate;
use crate::db::{devices, state};
use crate::logging;
use crate::service::sync::*;
use crate::service::sync_service::AppState;

pub async fn handle(
    request: Request<SyncHandshakeRequest>,
    state: &AppState,
) -> Result<Response<SyncHandshakeResponse>, Status> {
    let ctx = authenticate(request.metadata(), &state.pool)
        .await
        .map_err(Status::from)?;

    let req = request.into_inner();

    state.rate_limiter.acquire(ctx.account_id).await;

    devices::upsert_device(
        &state.pool,
        &req.device_id,
        ctx.account_id,
        if req.engine_version.is_empty() {
            None
        } else {
            Some(req.engine_version.as_str())
        },
    )
    .await
    .map_err(Status::from)?;

    let server_clock = state::get_clock(&state.pool, ctx.account_id)
        .await
        .map_err(Status::from)?;

    logging::audit_access(ctx.account_id, &req.device_id, "handshake");

    Ok(Response::new(SyncHandshakeResponse {
        server_clock: server_clock as u64,
        accepted: true,
        server_version: env!("CARGO_PKG_VERSION").to_string(),
        message: "ok".into(),
    }))
}
