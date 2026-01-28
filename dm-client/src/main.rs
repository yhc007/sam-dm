mod api;
mod config;
mod polling;
mod updater;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::Config;
use polling::PollingDaemon;

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

    // 설정 로드
    let config = Config::from_env().map_err(|e| {
        anyhow::anyhow!(
            "Missing environment variable: {}. Required: DM_SERVER_URL, DM_API_KEY",
            e
        )
    })?;

    // Polling 데몬 시작
    let daemon = PollingDaemon::new(config);
    daemon.run().await
}
