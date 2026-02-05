mod api;
mod config;
mod polling;
mod updater;
mod usb;

use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::Config;
use polling::PollingDaemon;

#[derive(Parser)]
#[command(name = "dm-client", version, about = "🦊 Sam DM Client - 원격 서비스 업데이트")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 서버 Polling 모드로 실행 (기본)
    Daemon,

    /// 로컬 파일/USB로 업데이트 적용
    Apply {
        /// 아티팩트 파일 경로 (.tar.gz)
        #[arg(short, long)]
        file: Option<String>,

        /// USB/디렉토리 경로 (manifest.json 자동 탐지)
        #[arg(short, long)]
        dir: Option<String>,

        /// 대상 버전 (manifest.json 없을 때 필수)
        #[arg(short, long)]
        version: Option<String>,

        /// SHA256 체크섬
        #[arg(short, long)]
        checksum: Option<String>,
    },

    /// 현재 버전 확인
    Status,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 로깅 초기화
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,dm_client=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // .env 파일 로드
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Daemon) {
        Commands::Daemon => {
            // 설정 로드 (서버 모드는 전체 설정 필요)
            let config = Config::from_env().map_err(|e| {
                anyhow::anyhow!(
                    "Missing environment variable: {}. Required: DM_SERVER_URL, DM_API_KEY",
                    e
                )
            })?;

            let daemon = PollingDaemon::new(config);
            daemon.run().await
        }

        Commands::Apply { file, dir, version, checksum } => {
            // Apply 모드는 서버 설정 없이도 동작
            let config = Config::from_env_optional();

            if let Some(dir_path) = dir {
                usb::apply_from_directory(&config, &dir_path)
            } else if let Some(file_path) = file {
                usb::apply_from_file(
                    &config,
                    &file_path,
                    version.as_deref(),
                    checksum.as_deref(),
                )
            } else {
                anyhow::bail!("--file 또는 --dir 중 하나를 지정해주세요.\n\n예시:\n  dm-client apply --dir /mnt/usb\n  dm-client apply --file /mnt/usb/update.tar.gz --version 1.0.0")
            }
        }

        Commands::Status => {
            let config = Config::from_env_optional();
            let version_file = std::path::Path::new(&config.service_dir).join(".dm-version");
            match std::fs::read_to_string(&version_file) {
                Ok(version) => println!("🦊 현재 버전: {}", version.trim()),
                Err(_) => println!("🦊 버전 정보 없음 (아직 설치되지 않음)"),
            }
            println!("   서비스 디렉토리: {}", config.service_dir);
            println!("   백업 디렉토리: {}", config.backup_dir);
            Ok(())
        }
    }
}
