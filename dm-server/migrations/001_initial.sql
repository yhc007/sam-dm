-- Sam DM Server 초기 스키마
-- PostgreSQL

-- 클라이언트 (타겟 서버) 테이블
CREATE TABLE IF NOT EXISTS clients (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    api_key VARCHAR(255) NOT NULL UNIQUE,
    current_version VARCHAR(50),
    target_version VARCHAR(50),
    last_seen TIMESTAMPTZ,
    status VARCHAR(50) NOT NULL DEFAULT 'offline',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_clients_api_key ON clients(api_key);
CREATE INDEX idx_clients_status ON clients(status);

-- 버전 테이블
CREATE TABLE IF NOT EXISTS versions (
    id UUID PRIMARY KEY,
    version VARCHAR(50) NOT NULL UNIQUE,
    artifact_path VARCHAR(500) NOT NULL,
    artifact_size BIGINT NOT NULL,
    checksum VARCHAR(64) NOT NULL,
    release_notes TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_versions_version ON versions(version);
CREATE INDEX idx_versions_is_active ON versions(is_active);

-- 업데이트 로그 테이블
CREATE TABLE IF NOT EXISTS update_logs (
    id UUID PRIMARY KEY,
    client_id UUID NOT NULL REFERENCES clients(id) ON DELETE CASCADE,
    from_version VARCHAR(50),
    to_version VARCHAR(50) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    error_message TEXT,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX idx_update_logs_client_id ON update_logs(client_id);
CREATE INDEX idx_update_logs_status ON update_logs(status);
CREATE INDEX idx_update_logs_started_at ON update_logs(started_at DESC);
