use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::Response,
};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::db;
use crate::AppState;

/// 아티팩트 다운로드
/// GET /api/artifacts/:version
pub async fn download_artifact(
    State(state): State<AppState>,
    Path(version): Path<String>,
) -> Result<Response<Body>, (StatusCode, String)> {
    // 버전 조회
    let ver = db::get_version(&state.pool, &version)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Version not found".to_string()))?;

    // 파일 경로
    let file_path = std::path::Path::new(&state.config.artifact_dir).join(&ver.artifact_path);

    // 파일 열기
    let file = File::open(&file_path)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "Artifact file not found".to_string()))?;

    // 스트리밍 응답
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", ver.artifact_path),
        )
        .header(header::CONTENT_LENGTH, ver.artifact_size)
        .header("X-Checksum-SHA256", ver.checksum)
        .body(body)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(response)
}
