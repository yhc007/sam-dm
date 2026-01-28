use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::process::Command;
use tar::Archive;
use tempfile::TempDir;

use crate::config::Config;

/// 서비스 업데이터
pub struct Updater {
    config: Config,
}

impl Updater {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// 체크섬 검증
    pub fn verify_checksum(&self, data: &[u8], expected: &str) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let actual = format!("{:x}", hasher.finalize());
        actual == expected
    }

    /// 현재 서비스 백업
    pub fn backup_current(&self, version: &str) -> Result<String> {
        let service_dir = Path::new(&self.config.service_dir);
        
        if !service_dir.exists() {
            tracing::warn!("Service directory does not exist, skipping backup");
            return Ok(String::new());
        }

        let backup_dir = Path::new(&self.config.backup_dir);
        fs::create_dir_all(backup_dir)?;

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_name = format!("backup_{}_{}", version, timestamp);
        let backup_path = backup_dir.join(&backup_name);

        tracing::info!("Creating backup at {:?}", backup_path);

        // Copy service directory to backup
        copy_dir_recursive(service_dir, &backup_path)?;

        Ok(backup_path.to_string_lossy().to_string())
    }

    /// 아티팩트 추출 및 설치
    pub fn extract_and_install(&self, data: &[u8]) -> Result<()> {
        let service_dir = Path::new(&self.config.service_dir);
        
        // Create temp directory for extraction
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path();

        tracing::info!("Extracting artifact to {:?}", temp_path);

        // Decompress and extract tar.gz
        let tar = GzDecoder::new(data);
        let mut archive = Archive::new(tar);
        archive.unpack(temp_path).context("Failed to extract archive")?;

        // Find the extracted content (might be in a subdirectory)
        let extracted_content = find_extracted_root(temp_path)?;

        // Clear existing service directory
        if service_dir.exists() {
            fs::remove_dir_all(service_dir)?;
        }
        fs::create_dir_all(service_dir)?;

        // Copy extracted content to service directory
        tracing::info!("Installing to {:?}", service_dir);
        copy_dir_recursive(&extracted_content, service_dir)?;

        Ok(())
    }

    /// 서비스 재시작
    pub fn restart_service(&self) -> Result<()> {
        tracing::info!("Restarting service: {}", self.config.restart_command);

        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", &self.config.restart_command])
                .output()?
        } else {
            Command::new("sh")
                .args(["-c", &self.config.restart_command])
                .output()?
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Restart command failed: {}", stderr);
        }

        tracing::info!("Service restarted successfully");
        Ok(())
    }

    /// 헬스 체크
    pub fn health_check(&self) -> Result<bool> {
        let Some(cmd) = &self.config.health_check_command else {
            tracing::info!("No health check command configured, assuming healthy");
            return Ok(true);
        };

        tracing::info!("Running health check: {}", cmd);

        // Wait a bit for service to start
        std::thread::sleep(std::time::Duration::from_secs(5));

        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", cmd])
                .output()?
        } else {
            Command::new("sh")
                .args(["-c", cmd])
                .output()?
        };

        Ok(output.status.success())
    }

    /// 백업에서 복원 (롤백)
    pub fn rollback(&self, backup_path: &str) -> Result<()> {
        if backup_path.is_empty() {
            anyhow::bail!("No backup available for rollback");
        }

        let backup_dir = Path::new(backup_path);
        let service_dir = Path::new(&self.config.service_dir);

        if !backup_dir.exists() {
            anyhow::bail!("Backup directory not found: {}", backup_path);
        }

        tracing::info!("Rolling back from {:?}", backup_dir);

        // Clear current service directory
        if service_dir.exists() {
            fs::remove_dir_all(service_dir)?;
        }
        fs::create_dir_all(service_dir)?;

        // Restore from backup
        copy_dir_recursive(backup_dir, service_dir)?;

        // Restart service
        self.restart_service()?;

        tracing::info!("Rollback completed successfully");
        Ok(())
    }
}

/// 디렉토리 재귀 복사
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// 추출된 루트 디렉토리 찾기
fn find_extracted_root(temp_path: &Path) -> Result<std::path::PathBuf> {
    let entries: Vec<_> = fs::read_dir(temp_path)?
        .filter_map(|e| e.ok())
        .collect();

    // If there's exactly one directory, use it as root
    if entries.len() == 1 && entries[0].file_type()?.is_dir() {
        return Ok(entries[0].path());
    }

    // Otherwise use the temp directory itself
    Ok(temp_path.to_path_buf())
}
