# Sam DM 🦊

OMA DM 기반 원격 서비스 업데이트 시스템

## 구조

```
sam-dm/
├── dm-server/     # DM 서버 (Rust + Axum + PostgreSQL)
└── dm-client/     # DM 클라이언트 (Rust)
```

## 기능

- **버전 관리**: Semver 기반 버전 관리
- **아티팩트 저장**: 빌드된 파일을 서버에 저장
- **원격 배포**: Polling 방식으로 클라이언트에 업데이트 명령
- **자동 롤백**: 업데이트 실패 시 이전 버전으로 복구

## DM Server 실행

### 1. PostgreSQL 준비

```bash
# 데이터베이스 생성
createdb sam_dm

# 마이그레이션 실행
psql -d sam_dm -f dm-server/migrations/001_initial.sql
```

### 2. 환경 설정

```bash
cd dm-server
cp .env.example .env
# .env 파일 수정
```

### 3. 빌드 & 실행

```bash
cargo build --release
cargo run
```

## API 엔드포인트

### 관리 API

| Method | Endpoint | 설명 |
|--------|----------|------|
| POST | `/api/clients` | 새 클라이언트 등록 |
| GET | `/api/clients` | 클라이언트 목록 |
| GET | `/api/clients/{id}` | 클라이언트 상세 |
| POST | `/api/clients/{id}/deploy` | 버전 배포 명령 |
| POST | `/api/versions` | 버전 업로드 (multipart) |
| GET | `/api/versions` | 버전 목록 |
| GET | `/api/versions/{version}` | 버전 상세 |
| GET | `/api/artifacts/{version}` | 아티팩트 다운로드 |

### 클라이언트 API

| Method | Endpoint | 설명 |
|--------|----------|------|
| POST | `/api/checkin` | 클라이언트 체크인 (Polling) |
| POST | `/api/update-result` | 업데이트 결과 보고 |

## 사용 예시

### 클라이언트 등록

```bash
curl -X POST http://localhost:3000/api/clients \
  -H "Content-Type: application/json" \
  -d '{"name": "production-server-1"}'
```

응답:
```json
{
  "id": "uuid...",
  "name": "production-server-1",
  "api_key": "generated-api-key..."
}
```

### 버전 업로드

```bash
curl -X POST http://localhost:3000/api/versions \
  -F "version=1.0.0" \
  -F "artifact=@./build.tar.gz" \
  -F "release_notes=Initial release"
```

### 배포 명령

```bash
curl -X POST http://localhost:3000/api/clients/{client-id}/deploy \
  -H "Content-Type: application/json" \
  -d '{"client_id": "uuid...", "version": "1.0.0"}'
```

### 클라이언트 체크인

```bash
curl -X POST http://localhost:3000/api/checkin \
  -H "X-API-Key: your-api-key" \
  -H "Content-Type: application/json" \
  -d '{"current_version": "0.9.0", "status": "online"}'
```

응답 (업데이트 필요 시):
```json
{
  "action": "update",
  "target_version": "1.0.0",
  "artifact_url": "/api/artifacts/1.0.0",
  "checksum": "sha256..."
}
```

## 라이센스

MIT
