use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

use crate::config::Config;
use crate::updater::Updater;

const VERSION_FILE: &str = ".dm-version";

/// USB manifest.json 구조
#[derive(Debug, Deserialize)]
pub struct UsbManifest {
    pub version: String,
    pub checksum: String,
    #[serde(default = "default_artifact")]
    pub artifact: String,
    pub release_notes: Option<String>,
}

fn default_artifact() -> String {
    "update.tar.gz".to_string()
}

/// USB/로컬 파일로 업데이트 수행
pub fn apply_from_file(
    config: &Config,
    file_path: &str,
    version: Option<&str>,
    checksum: Option<&str>,
) -> Result<()> {
    let updater = Updater::new(config.clone());
    let file = Path::new(file_path);

    if !file.exists() {
        anyhow::bail!("파일을 찾을 수 없습니다: {}", file_path);
    }

    // manifest.json 자동 탐지 (같은 디렉토리)
    let parent = file.parent().unwrap_or(Path::new("."));
    let manifest_path = parent.join("manifest.json");
    let manifest = if manifest_path.exists() {
        let data = fs::read_to_string(&manifest_path)
            .context("manifest.json 읽기 실패")?;
        Some(serde_json::from_str::<UsbManifest>(&data)
            .context("manifest.json 파싱 실패")?)
    } else {
        None
    };

    // 버전 결정 (CLI 인자 > manifest > 필수)
    let target_version = version
        .map(|v| v.to_string())
        .or_else(|| manifest.as_ref().map(|m| m.version.clone()))
        .ok_or_else(|| anyhow::anyhow!(
            "버전을 지정해주세요: --version 또는 manifest.json"
        ))?;

    // 체크섬 결정 (CLI 인자 > manifest > 스킵)
    let expected_checksum = checksum
        .map(|c| c.to_string())
        .or_else(|| manifest.as_ref().map(|m| m.checksum.clone()));

    // 현재 버전 읽기
    let version_file = Path::new(&config.service_dir).join(VERSION_FILE);
    let current_version = fs::read_to_string(&version_file)
        .ok()
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    tracing::info!("🦊 USB 업데이트 시작: {} -> {}", current_version, target_version);

    if let Some(notes) = manifest.as_ref().and_then(|m| m.release_notes.as_ref()) {
        tracing::info!("릴리즈 노트: {}", notes);
    }

    // 1. 파일 읽기
    tracing::info!("아티팩트 읽는 중: {}", file_path);
    let artifact_data = fs::read(file)
        .context("아티팩트 파일 읽기 실패")?;

    // 2. 체크섬 검증
    if let Some(ref expected) = expected_checksum {
        tracing::info!("체크섬 검증 중...");
        if !updater.verify_checksum(&artifact_data, expected) {
            anyhow::bail!("체크섬 불일치! 파일이 손상되었을 수 있습니다.");
        }
        tracing::info!("체크섬 검증 ✓");
    } else {
        tracing::warn!("체크섬 없이 진행합니다 (--checksum 또는 manifest.json 권장)");
    }

    // 3. 백업
    tracing::info!("현재 버전 백업 중...");
    let backup_path = updater.backup_current(&current_version)?;

    // 4. 설치
    tracing::info!("설치 중...");
    if let Err(e) = updater.extract_and_install(&artifact_data) {
        tracing::error!("설치 실패: {}", e);
        if !backup_path.is_empty() {
            tracing::info!("롤백 중...");
            updater.rollback(&backup_path)?;
        }
        return Err(e);
    }

    // 5. 버전 파일 업데이트
    fs::create_dir_all(&config.service_dir)?;
    fs::write(&version_file, &target_version)?;

    // 6. 서비스 재시작
    tracing::info!("서비스 재시작 중...");
    if let Err(e) = updater.restart_service() {
        tracing::error!("재시작 실패: {}", e);
        if !backup_path.is_empty() {
            tracing::info!("롤백 중...");
            updater.rollback(&backup_path)?;
            fs::write(&version_file, &current_version)?;
        }
        return Err(e);
    }

    // 7. 헬스 체크
    tracing::info!("헬스 체크 중...");
    match updater.health_check() {
        Ok(true) => {
            tracing::info!("헬스 체크 통과 ✓");
        }
        Ok(false) | Err(_) => {
            tracing::error!("헬스 체크 실패!");
            if !backup_path.is_empty() {
                tracing::info!("롤백 중...");
                updater.rollback(&backup_path)?;
                fs::write(&version_file, &current_version)?;
            }
            anyhow::bail!("헬스 체크 실패 - 롤백 완료");
        }
    }

    tracing::info!("✅ USB 업데이트 완료: {}", target_version);
    Ok(())
}

/// USB 경로에서 자동 탐지하여 업데이트
pub fn apply_from_directory(config: &Config, dir_path: &str) -> Result<()> {
    let dir = Path::new(dir_path);

    if !dir.exists() || !dir.is_dir() {
        anyhow::bail!("디렉토리를 찾을 수 없습니다: {}", dir_path);
    }

    // manifest.json 찾기
    let manifest_path = dir.join("manifest.json");
    if !manifest_path.exists() {
        anyhow::bail!(
            "manifest.json을 찾을 수 없습니다.\n\
             USB에 다음 파일이 필요합니다:\n\
             - manifest.json (버전, 체크섬 정보)\n\
             - update.tar.gz (아티팩트)"
        );
    }

    let manifest_data = fs::read_to_string(&manifest_path)?;
    let manifest: UsbManifest = serde_json::from_str(&manifest_data)
        .context("manifest.json 파싱 실패")?;

    let artifact_path = dir.join(&manifest.artifact);
    if !artifact_path.exists() {
        anyhow::bail!("아티팩트 파일을 찾을 수 없습니다: {}", manifest.artifact);
    }

    apply_from_file(
        config,
        artifact_path.to_str().unwrap(),
        Some(&manifest.version),
        Some(&manifest.checksum),
    )
}
