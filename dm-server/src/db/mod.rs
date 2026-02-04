pub mod models;

use anyhow::Result;
use chrono::Utc;
use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

pub use models::*;

/// 데이터베이스 연결 풀 생성
pub async fn create_pool(database_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;
    Ok(pool)
}

/// 클라이언트 등록
pub async fn register_client(pool: &PgPool, name: &str, api_key: &str, config: Option<&ClientConfig>) -> Result<Client> {
    let config_json = config.map(|c| serde_json::to_value(c).unwrap_or_default()).unwrap_or(serde_json::json!({}));
    
    let client = sqlx::query_as::<_, Client>(
        r#"
        INSERT INTO clients (id, name, api_key, status, config, created_at, updated_at)
        VALUES ($1, $2, $3, 'offline', $4, $5, $5)
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(name)
    .bind(api_key)
    .bind(config_json)
    .bind(Utc::now())
    .fetch_one(pool)
    .await?;

    Ok(client)
}

/// 클라이언트 설정 업데이트
pub async fn update_client_config(pool: &PgPool, client_id: Uuid, config: &ClientConfig) -> Result<()> {
    let config_json = serde_json::to_value(config)?;
    
    sqlx::query(
        r#"
        UPDATE clients
        SET config = $2, updated_at = $3
        WHERE id = $1
        "#,
    )
    .bind(client_id)
    .bind(config_json)
    .bind(Utc::now())
    .execute(pool)
    .await?;

    Ok(())
}

/// API Key로 클라이언트 조회
pub async fn get_client_by_api_key(pool: &PgPool, api_key: &str) -> Result<Option<Client>> {
    let client = sqlx::query_as::<_, Client>("SELECT * FROM clients WHERE api_key = $1")
        .bind(api_key)
        .fetch_optional(pool)
        .await?;
    Ok(client)
}

/// 클라이언트 ID로 조회
pub async fn get_client_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Client>> {
    let client = sqlx::query_as::<_, Client>("SELECT * FROM clients WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(client)
}

/// 모든 클라이언트 조회
pub async fn get_all_clients(pool: &PgPool) -> Result<Vec<Client>> {
    let clients = sqlx::query_as::<_, Client>("SELECT * FROM clients ORDER BY created_at DESC")
        .fetch_all(pool)
        .await?;
    Ok(clients)
}

/// 클라이언트 체크인 업데이트
pub async fn update_client_checkin(
    pool: &PgPool,
    client_id: Uuid,
    current_version: Option<&str>,
    status: &str,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE clients
        SET current_version = COALESCE($2, current_version),
            status = $3,
            last_seen = $4,
            updated_at = $4
        WHERE id = $1
        "#,
    )
    .bind(client_id)
    .bind(current_version)
    .bind(status)
    .bind(Utc::now())
    .execute(pool)
    .await?;

    Ok(())
}

/// 클라이언트 타겟 버전 설정
pub async fn set_client_target_version(
    pool: &PgPool,
    client_id: Uuid,
    target_version: &str,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE clients
        SET target_version = $2, updated_at = $3
        WHERE id = $1
        "#,
    )
    .bind(client_id)
    .bind(target_version)
    .bind(Utc::now())
    .execute(pool)
    .await?;

    Ok(())
}

/// 버전 생성
pub async fn create_version(
    pool: &PgPool,
    version: &str,
    artifact_path: &str,
    artifact_size: i64,
    checksum: &str,
    release_notes: Option<&str>,
) -> Result<Version> {
    let ver = sqlx::query_as::<_, Version>(
        r#"
        INSERT INTO versions (id, version, artifact_path, artifact_size, checksum, release_notes, is_active, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, true, $7)
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(version)
    .bind(artifact_path)
    .bind(artifact_size)
    .bind(checksum)
    .bind(release_notes)
    .bind(Utc::now())
    .fetch_one(pool)
    .await?;

    Ok(ver)
}

/// 버전 조회
pub async fn get_version(pool: &PgPool, version: &str) -> Result<Option<Version>> {
    let ver = sqlx::query_as::<_, Version>("SELECT * FROM versions WHERE version = $1")
        .bind(version)
        .fetch_optional(pool)
        .await?;
    Ok(ver)
}

/// 모든 버전 조회
pub async fn get_all_versions(pool: &PgPool) -> Result<Vec<Version>> {
    let versions =
        sqlx::query_as::<_, Version>("SELECT * FROM versions ORDER BY created_at DESC")
            .fetch_all(pool)
            .await?;
    Ok(versions)
}

/// 업데이트 로그 생성
pub async fn create_update_log(
    pool: &PgPool,
    client_id: Uuid,
    from_version: Option<&str>,
    to_version: &str,
) -> Result<UpdateLog> {
    let log = sqlx::query_as::<_, UpdateLog>(
        r#"
        INSERT INTO update_logs (id, client_id, from_version, to_version, status, started_at)
        VALUES ($1, $2, $3, $4, 'pending', $5)
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(client_id)
    .bind(from_version)
    .bind(to_version)
    .bind(Utc::now())
    .fetch_one(pool)
    .await?;

    Ok(log)
}

/// 업데이트 로그 상태 업데이트
pub async fn update_log_status(
    pool: &PgPool,
    log_id: Uuid,
    status: &str,
    error_message: Option<&str>,
) -> Result<()> {
    let completed_at = if status == "completed" || status == "failed" || status == "rolled_back" {
        Some(Utc::now())
    } else {
        None
    };

    sqlx::query(
        r#"
        UPDATE update_logs
        SET status = $2, error_message = $3, completed_at = $4
        WHERE id = $1
        "#,
    )
    .bind(log_id)
    .bind(status)
    .bind(error_message)
    .bind(completed_at)
    .execute(pool)
    .await?;

    Ok(())
}
