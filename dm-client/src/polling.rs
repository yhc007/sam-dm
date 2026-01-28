use anyhow::Result;
use std::fs;
use std::path::Path;
use tokio::time::{sleep, Duration};

use crate::api::DmApiClient;
use crate::config::Config;
use crate::updater::Updater;

const VERSION_FILE: &str = ".dm-version";

/// Polling 기반 업데이트 루프
pub struct PollingDaemon {
    config: Config,
    api: DmApiClient,
    updater: Updater,
}

impl PollingDaemon {
    pub fn new(config: Config) -> Self {
        let api = DmApiClient::new(&config.server_url, &config.api_key);
        let updater = Updater::new(config.clone());
        
        Self { config, api, updater }
    }

    /// 현재 버전 읽기
    fn read_current_version(&self) -> Option<String> {
        let version_file = Path::new(&self.config.service_dir).join(VERSION_FILE);
        fs::read_to_string(version_file).ok().map(|s| s.trim().to_string())
    }

    /// 현재 버전 저장
    fn write_current_version(&self, version: &str) -> Result<()> {
        let version_file = Path::new(&self.config.service_dir).join(VERSION_FILE);
        fs::create_dir_all(&self.config.service_dir)?;
        fs::write(version_file, version)?;
        Ok(())
    }

    /// 업데이트 실행
    async fn perform_update(&self, target_version: &str, artifact_url: &str, checksum: &str) -> Result<()> {
        let current_version = self.read_current_version().unwrap_or_else(|| "unknown".to_string());
        
        tracing::info!("Starting update: {} -> {}", current_version, target_version);

        // 1. 아티팩트 다운로드
        tracing::info!("Downloading artifact...");
        let artifact_data = self.api.download_artifact(artifact_url).await?;

        // 2. 체크섬 검증
        tracing::info!("Verifying checksum...");
        if !self.updater.verify_checksum(&artifact_data, checksum) {
            anyhow::bail!("Checksum verification failed!");
        }
        tracing::info!("Checksum verified ✓");

        // 3. 현재 버전 백업
        tracing::info!("Creating backup...");
        let backup_path = self.updater.backup_current(&current_version)?;

        // 4. 추출 및 설치
        tracing::info!("Extracting and installing...");
        if let Err(e) = self.updater.extract_and_install(&artifact_data) {
            tracing::error!("Installation failed: {}", e);
            if !backup_path.is_empty() {
                tracing::info!("Attempting rollback...");
                self.updater.rollback(&backup_path)?;
            }
            return Err(e);
        }

        // 5. 버전 파일 업데이트
        self.write_current_version(target_version)?;

        // 6. 서비스 재시작
        tracing::info!("Restarting service...");
        if let Err(e) = self.updater.restart_service() {
            tracing::error!("Restart failed: {}", e);
            if !backup_path.is_empty() {
                tracing::info!("Attempting rollback...");
                self.updater.rollback(&backup_path)?;
                self.write_current_version(&current_version)?;
            }
            return Err(e);
        }

        // 7. 헬스 체크
        tracing::info!("Running health check...");
        match self.updater.health_check() {
            Ok(true) => {
                tracing::info!("Health check passed ✓");
            }
            Ok(false) | Err(_) => {
                tracing::error!("Health check failed!");
                if !backup_path.is_empty() {
                    tracing::info!("Attempting rollback...");
                    self.updater.rollback(&backup_path)?;
                    self.write_current_version(&current_version)?;
                }
                anyhow::bail!("Health check failed after update");
            }
        }

        tracing::info!("Update completed successfully: {}", target_version);
        Ok(())
    }

    /// 메인 Polling 루프
    pub async fn run(&self) -> Result<()> {
        tracing::info!("🦊 Sam DM Client starting...");
        tracing::info!("Server: {}", self.config.server_url);
        tracing::info!("Poll interval: {}s", self.config.poll_interval_secs);
        tracing::info!("Service dir: {}", self.config.service_dir);

        loop {
            let current_version = self.read_current_version();
            
            tracing::debug!(
                "Checking in (current version: {})",
                current_version.as_deref().unwrap_or("none")
            );

            // 서버에 체크인
            match self.api.checkin(current_version.as_deref(), "online").await {
                Ok(response) => {
                    if response.action == "update" {
                        let target = response.target_version.as_deref().unwrap_or("unknown");
                        let artifact_url = response.artifact_url.as_deref().unwrap_or("");
                        let checksum = response.checksum.as_deref().unwrap_or("");

                        tracing::info!("Update available: {}", target);

                        match self.perform_update(target, artifact_url, checksum).await {
                            Ok(()) => {
                                // 성공 보고
                                if let Err(e) = self.api.report_result(target, true, None).await {
                                    tracing::error!("Failed to report success: {}", e);
                                }
                            }
                            Err(e) => {
                                // 실패 보고
                                tracing::error!("Update failed: {}", e);
                                if let Err(e2) = self.api.report_result(target, false, Some(&e.to_string())).await {
                                    tracing::error!("Failed to report failure: {}", e2);
                                }
                            }
                        }
                    } else {
                        tracing::debug!("No update required");
                    }
                }
                Err(e) => {
                    tracing::error!("Checkin failed: {}", e);
                }
            }

            // 다음 폴링까지 대기
            sleep(Duration::from_secs(self.config.poll_interval_secs)).await;
        }
    }
}
