use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// 체크인 요청
#[derive(Debug, Serialize)]
pub struct CheckinRequest {
    pub current_version: Option<String>,
    pub status: String,
}

/// 체크인 응답
#[derive(Debug, Deserialize)]
pub struct CheckinResponse {
    pub action: String, // "none" or "update"
    pub target_version: Option<String>,
    pub artifact_url: Option<String>,
    pub checksum: Option<String>,
}

/// 업데이트 결과 보고
#[derive(Debug, Serialize)]
pub struct UpdateResultRequest {
    pub version: String,
    pub success: bool,
    pub error_message: Option<String>,
}

/// DM Server API 클라이언트
pub struct DmApiClient {
    client: Client,
    server_url: String,
    api_key: String,
}

impl DmApiClient {
    pub fn new(server_url: &str, api_key: &str) -> Self {
        Self {
            client: Client::new(),
            server_url: server_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
        }
    }

    /// 서버에 체크인 (Polling)
    pub async fn checkin(&self, current_version: Option<&str>, status: &str) -> Result<CheckinResponse> {
        let url = format!("{}/api/checkin", self.server_url);
        
        let req = CheckinRequest {
            current_version: current_version.map(|s| s.to_string()),
            status: status.to_string(),
        };

        let response = self.client
            .post(&url)
            .header("X-API-Key", &self.api_key)
            .json(&req)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Checkin failed: {} - {}", status, text);
        }

        let checkin_response: CheckinResponse = response.json().await?;
        Ok(checkin_response)
    }

    /// 아티팩트 다운로드
    pub async fn download_artifact(&self, artifact_url: &str) -> Result<Vec<u8>> {
        let url = if artifact_url.starts_with("http") {
            artifact_url.to_string()
        } else {
            format!("{}{}", self.server_url, artifact_url)
        };

        tracing::info!("Downloading artifact from {}", url);

        let response = self.client
            .get(&url)
            .header("X-API-Key", &self.api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            anyhow::bail!("Download failed: {}", status);
        }

        let bytes = response.bytes().await?.to_vec();
        Ok(bytes)
    }

    /// 업데이트 결과 보고
    pub async fn report_result(&self, version: &str, success: bool, error_message: Option<&str>) -> Result<()> {
        let url = format!("{}/api/update-result", self.server_url);
        
        let req = UpdateResultRequest {
            version: version.to_string(),
            success,
            error_message: error_message.map(|s| s.to_string()),
        };

        let response = self.client
            .post(&url)
            .header("X-API-Key", &self.api_key)
            .json(&req)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Report failed: {} - {}", status, text);
        }

        Ok(())
    }
}
