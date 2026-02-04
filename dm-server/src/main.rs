mod api;
mod config;
mod db;

use axum::{
    routing::{get, post, put},
    Router,
};
use sqlx::PgPool;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::Config;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 로깅 초기화
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,dm_server=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // .env 파일 로드
    dotenvy::dotenv().ok();

    // 설정 로드
    let config = Config::from_env()?;
    tracing::info!("Starting DM Server on {}", config.server_addr());

    // 데이터베이스 연결
    let pool = db::create_pool(&config.database_url).await?;
    tracing::info!("Connected to database");

    // 아티팩트 디렉토리 생성
    tokio::fs::create_dir_all(&config.artifact_dir).await?;
    tracing::info!("Artifact directory: {}", config.artifact_dir);

    let state = AppState {
        pool,
        config: Arc::new(config.clone()),
    };

    // CORS 설정
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // 라우터 설정
    let app = Router::new()
        // 관리 API
        .route("/api/clients", get(api::list_clients).post(api::register_client))
        .route("/api/clients/:id", get(api::get_client))
        .route("/api/clients/:id/config", put(api::update_client_config))
        .route("/api/clients/:id/deploy", post(api::deploy_to_client))
        .route("/api/versions", get(api::list_versions).post(api::upload_version))
        .route("/api/versions/:version", get(api::get_version))
        .route("/api/artifacts/:version", get(api::download_artifact))
        // 클라이언트 Polling API
        .route("/api/checkin", post(api::checkin))
        .route("/api/update-result", post(api::report_update_result))
        // Health check
        .route("/health", get(|| async { "OK" }))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // 서버 시작
    let listener = tokio::net::TcpListener::bind(config.server_addr()).await?;
    tracing::info!("🦊 Sam DM Server is running!");
    axum::serve(listener, app).await?;

    Ok(())
}
