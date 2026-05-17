use std::pin::Pin;

use tokio_stream::Stream;
use tonic::{Request, Response, Status};

use crate::auth::authenticate;
use crate::db::packs;
use crate::db::validation;
use crate::logging;
use crate::service::sync::*;
use crate::service::sync_service::AppState;
use crate::storage;

pub async fn handle(
    request: Request<PackPullRequest>,
    state: &AppState,
) -> Result<Response<Pin<Box<dyn Stream<Item = Result<PackPullResponse, Status>> + Send>>>, Status> {
    let ctx = authenticate(request.metadata(), &state.pool)
        .await
        .map_err(Status::from)?;

    let req = request.into_inner();

    let _permit = state.rate_limiter.acquire(ctx.account_id).await;

    validation::validate_device_id(&req.device_id).map_err(Status::from)?;

    let pack = packs::lookup_pack(&state.pool, ctx.account_id, req.pack_id as i64)
        .await
        .map_err(Status::from)?
        .ok_or_else(|| Status::not_found("pack not found"))?;

    if pack.state != "ready" {
        return Err(Status::unavailable("pack not yet available"));
    }

    let bytes = storage::download_pack(
        &state.r2_client,
        &state.r2_bucket,
        ctx.account_id,
        req.pack_id as i64,
    )
    .await
    .map_err(Status::from)?;

    logging::audit_access(ctx.account_id, &req.device_id, "pull_pack");

    let total = bytes.len() as u64;
    let chunk_size = 65536usize;

    let (tx, rx) = tokio::sync::mpsc::channel(8);

    tokio::spawn(async move {
        let meta = PackPullResponse {
            pack_id: req.pack_id,
            total_bytes: total,
            chunk_data: vec![],
        };
        if tx.send(Ok(meta)).await.is_err() {
            return;
        }

        for chunk in bytes.chunks(chunk_size) {
            let resp = PackPullResponse {
                pack_id: req.pack_id,
                total_bytes: total,
                chunk_data: chunk.to_vec(),
            };
            if tx.send(Ok(resp)).await.is_err() {
                return;
            }
        }
    });

    Ok(Response::new(Box::pin(
        tokio_stream::wrappers::ReceiverStream::new(rx),
    )))
}
