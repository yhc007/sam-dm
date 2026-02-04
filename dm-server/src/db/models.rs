use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 클라이언트 설정
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientConfig {
    #[serde(default)]
    pub service_dir: Option<String>,
    #[serde(default)]
    pub restart_command: Option<String>,
    #[serde(default)]
    pub pre_update_script: Option<String>,
    #[serde(default)]
    pub post_update_script: Option<String>,
    #[serde(default)]
    pub health_check_url: Option<String>,
    #[serde(default)]
    pub health_check_timeout: Option<i32>,
    #[serde(default)]
    pub rollback_on_failure: Option<bool>,
}

/// 등록된 클라이언트 (타겟 서버)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Client {
    pub id: Uuid,
    pub name: String,
    pub api_key: String,
    pub current_version: Option<String>,
    pub target_version: Option<String>,
    pub last_seen: Option<DateTime<Utc>>,
    pub status: String, // "online", "offline", "updating", "error"
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[sqlx(default)]
    pub config: sqlx::types::Json<ClientConfig>,
}

/// 버전 정보
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Version {
    pub id: Uuid,
    pub version: String,          // semver: "1.2.3"
    pub artifact_path: String,    // 파일 경로
    pub artifact_size: i64,       // 파일 크기 (bytes)
    pub checksum: String,         // SHA256 해시
    pub release_notes: Option<String>,
    pub is_active: bool,          // 배포 가능 여부
    pub created_at: DateTime<Utc>,
}

/// 업데이트 기록
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UpdateLog {
    pub id: Uuid,
    pub client_id: Uuid,
    pub from_version: Option<String>,
    pub to_version: String,
    pub status: String, // "pending", "downloading", "installing", "completed", "failed", "rolled_back"
    pub error_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// 클라이언트 체크인 요청
#[derive(Debug, Deserialize)]
pub struct CheckinRequest {
    pub current_version: Option<String>,
    pub status: String,
}

/// 클라이언트 체크인 응답
#[derive(Debug, Serialize)]
pub struct CheckinResponse {
    pub action: String, // "none", "update"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<ClientConfig>,
}

/// 새 클라이언트 등록 요청
#[derive(Debug, Deserialize)]
pub struct RegisterClientRequest {
    pub name: String,
    #[serde(default)]
    pub config: Option<ClientConfig>,
}

/// 클라이언트 설정 업데이트 요청
#[derive(Debug, Deserialize)]
pub struct UpdateClientConfigRequest {
    pub config: ClientConfig,
}

/// 새 클라이언트 등록 응답
#[derive(Debug, Serialize)]
pub struct RegisterClientResponse {
    pub id: Uuid,
    pub name: String,
    pub api_key: String,
}

/// 버전 배포 명령
#[derive(Debug, Deserialize)]
pub struct DeployRequest {
    pub version: String,
}

/// 업데이트 결과 보고
#[derive(Debug, Deserialize)]
pub struct UpdateResultRequest {
    pub version: String,
    pub success: bool,
    pub error_message: Option<String>,
}
