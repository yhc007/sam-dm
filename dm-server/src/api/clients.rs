use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::db::{self, RegisterClientRequest, RegisterClientResponse, UpdateClientConfigRequest};
use crate::AppState;

/// API Key 생성
fn generate_api_key() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, &bytes)
}

/// 새 클라이언트 등록
/// POST /api/clients
pub async fn register_client(
    State(state): State<AppState>,
    Json(req): Json<RegisterClientRequest>,
) -> Result<Json<RegisterClientResponse>, (StatusCode, String)> {
    let api_key = generate_api_key();

    let client = db::register_client(&state.pool, &req.name, &api_key, req.config.as_ref())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(RegisterClientResponse {
        id: client.id,
        name: client.name,
        api_key,
    }))
}

/// 클라이언트 설정 업데이트
/// PUT /api/clients/:id/config
pub async fn update_client_config(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateClientConfigRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // 클라이언트 존재 확인
    let _client = db::get_client_by_id(&state.pool, id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Client not found".to_string()))?;

    // 설정 업데이트
    db::update_client_config(&state.pool, id, &req.config)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({
        "message": "Config updated",
        "client_id": id
    })))
}

/// 모든 클라이언트 조회
/// GET /api/clients
pub async fn list_clients(
    State(state): State<AppState>,
) -> Result<Json<Vec<db::Client>>, (StatusCode, String)> {
    let clients = db::get_all_clients(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(clients))
}

/// 클라이언트 조회
/// GET /api/clients/:id
pub async fn get_client(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<db::Client>, (StatusCode, String)> {
    let client = db::get_client_by_id(&state.pool, id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Client not found".to_string()))?;

    Ok(Json(client))
}

/// 클라이언트에 버전 배포 명령
/// POST /api/clients/:id/deploy
pub async fn deploy_to_client(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<db::DeployRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // 클라이언트 존재 확인
    let _client = db::get_client_by_id(&state.pool, id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Client not found".to_string()))?;

    // 버전 존재 확인
    let _version = db::get_version(&state.pool, &req.version)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Version not found".to_string()))?;

    // 타겟 버전 설정
    db::set_client_target_version(&state.pool, id, &req.version)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({
        "message": "Deploy command queued",
        "client_id": id,
        "target_version": req.version
    })))
}
