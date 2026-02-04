use axum::{
    extract::State,
    http::{header::HeaderMap, StatusCode},
    Json,
};

use crate::db::{self, CheckinRequest, CheckinResponse, UpdateResultRequest};
use crate::AppState;

/// API Key 추출
fn extract_api_key(headers: &HeaderMap) -> Option<String> {
    headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// 클라이언트 체크인 (Polling)
/// POST /api/checkin
/// Header: X-API-Key
pub async fn checkin(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CheckinRequest>,
) -> Result<Json<CheckinResponse>, (StatusCode, String)> {
    // API Key 확인
    let api_key = extract_api_key(&headers)
        .ok_or((StatusCode::UNAUTHORIZED, "X-API-Key header required".to_string()))?;

    // 클라이언트 조회
    let client = db::get_client_by_api_key(&state.pool, &api_key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::UNAUTHORIZED, "Invalid API key".to_string()))?;

    // 체크인 업데이트
    db::update_client_checkin(
        &state.pool,
        client.id,
        req.current_version.as_deref(),
        &req.status,
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 업데이트 필요 여부 확인
    let needs_update = match (&client.target_version, &req.current_version) {
        (Some(target), Some(current)) => target != current,
        (Some(_), None) => true,
        _ => false,
    };

    // 클라이언트 설정
    let client_config = client.config.0.clone();
    let config_option = if client_config.service_dir.is_some() || client_config.restart_command.is_some() {
        Some(client_config)
    } else {
        None
    };

    if needs_update {
        let target_version = client.target_version.unwrap();
        
        // 버전 정보 조회
        let version = db::get_version(&state.pool, &target_version)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if let Some(ver) = version {
            // 업데이트 로그 생성
            db::create_update_log(
                &state.pool,
                client.id,
                req.current_version.as_deref(),
                &target_version,
            )
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            return Ok(Json(CheckinResponse {
                action: "update".to_string(),
                target_version: Some(target_version),
                artifact_url: Some(format!("/api/artifacts/{}", ver.version)),
                checksum: Some(ver.checksum),
                config: config_option,
            }));
        }
    }

    Ok(Json(CheckinResponse {
        action: "none".to_string(),
        target_version: None,
        artifact_url: None,
        checksum: None,
        config: config_option,
    }))
}

/// 업데이트 결과 보고
/// POST /api/update-result
/// Header: X-API-Key
pub async fn report_update_result(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UpdateResultRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // API Key 확인
    let api_key = extract_api_key(&headers)
        .ok_or((StatusCode::UNAUTHORIZED, "X-API-Key header required".to_string()))?;

    // 클라이언트 조회
    let client = db::get_client_by_api_key(&state.pool, &api_key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::UNAUTHORIZED, "Invalid API key".to_string()))?;

    if req.success {
        // 성공: current_version 업데이트, target_version 클리어
        sqlx::query(
            r#"
            UPDATE clients
            SET current_version = $2, target_version = NULL, status = 'online', updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(client.id)
        .bind(&req.version)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(Json(serde_json::json!({
            "message": "Update success recorded",
            "version": req.version
        })))
    } else {
        // 실패: status를 error로
        sqlx::query(
            r#"
            UPDATE clients
            SET status = 'error', updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(client.id)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(Json(serde_json::json!({
            "message": "Update failure recorded",
            "version": req.version,
            "error": req.error_message
        })))
    }
}
