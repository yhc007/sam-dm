use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    Json,
};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::db::{self, Version};
use crate::AppState;

/// 버전 목록 조회
/// GET /api/versions
pub async fn list_versions(
    State(state): State<AppState>,
) -> Result<Json<Vec<Version>>, (StatusCode, String)> {
    let versions = db::get_all_versions(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(versions))
}

/// 버전 상세 조회
/// GET /api/versions/:version
pub async fn get_version(
    State(state): State<AppState>,
    Path(version): Path<String>,
) -> Result<Json<Version>, (StatusCode, String)> {
    let ver = db::get_version(&state.pool, &version)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Version not found".to_string()))?;

    Ok(Json(ver))
}

/// 새 버전 업로드
/// POST /api/versions
/// multipart form: version, artifact (file), release_notes (optional)
pub async fn upload_version(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<Version>, (StatusCode, String)> {
    let mut version_str: Option<String> = None;
    let mut release_notes: Option<String> = None;
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;

    // Parse multipart form
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "version" => {
                version_str = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?,
                );
            }
            "release_notes" => {
                release_notes = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?,
                );
            }
            "artifact" => {
                file_name = field.file_name().map(|s| s.to_string());
                file_data = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
                        .to_vec(),
                );
            }
            _ => {}
        }
    }

    let version_str =
        version_str.ok_or((StatusCode::BAD_REQUEST, "version field required".to_string()))?;
    let file_data =
        file_data.ok_or((StatusCode::BAD_REQUEST, "artifact file required".to_string()))?;

    // Validate semver
    semver::Version::parse(&version_str)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid semver: {}", e)))?;

    // Check if version already exists
    if db::get_version(&state.pool, &version_str)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .is_some()
    {
        return Err((
            StatusCode::CONFLICT,
            format!("Version {} already exists", version_str),
        ));
    }

    // Calculate checksum
    let mut hasher = Sha256::new();
    hasher.update(&file_data);
    let checksum = format!("{:x}", hasher.finalize());

    // Save file
    let file_ext = file_name
        .as_ref()
        .and_then(|n| n.rsplit('.').next())
        .unwrap_or("tar.gz");
    let artifact_filename = format!("{}.{}", version_str, file_ext);
    let artifact_path: PathBuf = [&state.config.artifact_dir, &artifact_filename]
        .iter()
        .collect();

    // Ensure artifact directory exists
    fs::create_dir_all(&state.config.artifact_dir)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut file = fs::File::create(&artifact_path)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    file.write_all(&file_data)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Save to database
    let version = db::create_version(
        &state.pool,
        &version_str,
        &artifact_filename,
        file_data.len() as i64,
        &checksum,
        release_notes.as_deref(),
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(version))
}
