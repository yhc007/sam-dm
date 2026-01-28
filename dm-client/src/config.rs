use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    /// DM Server URL (e.g., "http://localhost:3000")
    pub server_url: String,
    
    /// API Key for authentication
    pub api_key: String,
    
    /// Polling interval in seconds
    pub poll_interval_secs: u64,
    
    /// Service directory (where the Next.js app lives)
    pub service_dir: String,
    
    /// Backup directory for rollback
    pub backup_dir: String,
    
    /// Command to restart the service
    pub restart_command: String,
    
    /// Command to check service health
    pub health_check_command: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, env::VarError> {
        Ok(Self {
            server_url: env::var("DM_SERVER_URL")?,
            api_key: env::var("DM_API_KEY")?,
            poll_interval_secs: env::var("DM_POLL_INTERVAL")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
            service_dir: env::var("DM_SERVICE_DIR")
                .unwrap_or_else(|_| "./service".to_string()),
            backup_dir: env::var("DM_BACKUP_DIR")
                .unwrap_or_else(|_| "./backups".to_string()),
            restart_command: env::var("DM_RESTART_COMMAND")
                .unwrap_or_else(|_| "pm2 restart all".to_string()),
            health_check_command: env::var("DM_HEALTH_CHECK_COMMAND").ok(),
        })
    }
}
