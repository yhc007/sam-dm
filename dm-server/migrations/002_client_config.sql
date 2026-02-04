-- 클라이언트 설정 컬럼 추가
ALTER TABLE clients ADD COLUMN IF NOT EXISTS config JSONB DEFAULT '{}';

-- 설정 예시:
-- {
--   "service_dir": "/var/www/my-app",
--   "restart_command": "systemctl restart my-app",
--   "pre_update_script": "./backup.sh",
--   "post_update_script": "./migrate.sh",
--   "health_check_url": "http://localhost:3000/health",
--   "health_check_timeout": 30,
--   "rollback_on_failure": true
-- }

COMMENT ON COLUMN clients.config IS 'Client configuration for updates (service_dir, restart_command, scripts, etc.)';
