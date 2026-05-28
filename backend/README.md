# Crucible Backend

High-performance Rust backend for the Crucible smart contract testing platform, providing performance profiling, mock service layers, specialized serialization utilities, and robust background monitoring.

## 🚀 Tech Stack
- **Web Framework**: Axum (async Rust)
- **Runtime**: Tokio
- **Database**: PostgreSQL (via SQLx 0.8)
- **Caching & Jobs**: Redis (via Apalis)
- **Observability**: OpenTelemetry + Tracing
- **API Documentation**: Utoipa (Swagger UI)

---

## 🏗️ Architecture

```text
┌──────────────┐     ┌──────────────────────┐     ┌────────────────┐
│   Clients    │────▶│   Axum HTTP Server   │────▶│  PostgreSQL 16 │
│  (port 8080) │     │                      │     │  (port 5432)   │
└──────────────┘     │  Middleware Stack:   │     └────────────────┘
                     │  ├─ CORS             │
                     │  ├─ Tracing          │     ┌────────────────┐
                     │  ├─ Compression      │────▶│   Redis 7      │
                     │  └─ Request ID       │     │  (port 6379)   │
                     └──────────────────────┘     └────────────────┘
```

---

## ⚡ Quick Start

### Prerequisites
- [Docker](https://docs.docker.com/get-docker/) ≥ 24.0
- [Docker Compose](https://docs.docker.com/compose/install/) ≥ 2.20
- [Rust](https://rustup.rs/) ≥ 1.78 (for local development)

### Starting Services
```bash
cd backend
cp .env.example .env

# Start all core services (app, postgres, redis)
docker compose up -d

# Check service health
curl http://localhost:8080/health
```

### Local Development (without Docker)
Run Postgres and Redis in Docker, but the Rust app natively:
```bash
docker compose up -d postgres redis
export DATABASE_URL=postgres://crucible:crucible_secret@localhost:5432/crucible_db
export REDIS_URL=redis://:crucible_redis_secret@localhost:6379/0
cargo run
```

---

## ⚙️ Configuration

This application uses a layered configuration system. Base values and environment-specific tunings are compiled directly into the binary, ensuring safe fallbacks. Infrastructure secrets and dynamic overrides are provided securely at runtime via environment variables.

### Environment Variables

| Variable | Default | Required in Prod? | Description |
|---|---|---|---|
| `APP_ENV` | `development` | Yes | `development`, `staging`, or `production` |
| `APP_SERVER__PORT` | (from TOML) | No | HTTP server listen port. |
| `APP_SERVER__TLS__CERT_PATH` | None | Yes | Path to the TLS certificate chain. |
| `APP_SERVER__TLS__KEY_PATH` | None | Yes | Path to the TLS private key. **(SENSITIVE)** |
| `APP_DATABASE__URL` | None | Yes | PostgreSQL connection string. **(SENSITIVE)** |
| `APP_REDIS__URL` | None | Yes | Redis connection string. **(SENSITIVE)** |

### Configuration Hot-Reload

The backend supports atomic configuration hot-reloading without restarting the server via `ArcSwap`.

```bash
# Trigger a reload from the HTTP endpoint
curl -X POST http://localhost:8080/api/config/reload
```

---

## 📡 API Endpoints

Crucible uses a typed contract system for all API endpoints to ensure consistency.

| Category | Method | Path | Description |
|---|---|---|---|
| **Health** | `GET` | `/health` | Health check (DB + Redis) |
| **Health** | `GET` | `/api/status` | System health summary and recovery status |
| **Dashboard**| `GET` | `/api/v1/dashboard/metrics`| Dashboard aggregated metrics with Redis caching |
| **Errors** | `GET` | `/api/v1/errors/dashboard/build-errors`| Returns build error analytics |
| **Config** | `GET` | `/api/config` | Retrieve current configuration (sanitized) |
| **Config** | `POST` | `/api/config/reload` | Trigger configuration hot-reload |
| **Docs** | `GET` | `/swagger-ui` | Interactive API documentation |

---

## 🔭 Observability & Tracing

Production-grade distributed tracing with OTLP exporter and Jaeger integration.

1. **Start Jaeger**: `docker-compose -f docker-compose-jaeger.yml up -d`
2. **Run Backend**: `export OTLP_ENDPOINT=http://localhost:4317; cargo run -p backend`
3. **View Traces**: Open `http://localhost:16686`

Spans from every `#[tracing::instrument]`-annotated function are exported with **< 1% latency overhead**.

---

## 🛠️ Background Services & Features

### 1. Build System Metrics Exporter
Tracks compilation times, dependency counts, cache hit rates, and resource usage. Includes durable storage in PostgreSQL and high-performance caching in Redis.

### 2. Critical Error Alerting
Dispatches notifications when a critical condition fires (e.g., `db_down`). Deduplicates within a configurable cooldown window and publishes to Redis pub/sub.

### 3. Feature Flags
Stored in PostgreSQL and cached in Redis with a 5-minute TTL.

---

## 🧪 Testing

This project utilizes highly isolated, in-process integration testing leveraging Axum's `oneshot` capability.

```bash
# Unit tests
cargo test -p backend --lib

# Integration tests (requires PostgreSQL and Redis)
cargo test -p backend --test integration_tests
```

### Test Database Isolation Strategy
We utilize **Isolated Schemas**. For every `#[tokio::test]`, the `TestContext` dynamically creates a completely isolated PostgreSQL schema (e.g. `test_a1b2c3d4...`) and maps the SQLx connection pool strictly to that `search_path`.
- **Parallelization**: Tests run fully concurrently.
- **Safety**: The schema is dropped entirely on `Drop`.

### Integration Test Framework Example
You can utilize the `ApiTestClient` located in `src/test_utils/client.rs`.

```rust
use crate::test_utils::{setup, client::ApiTestClient};
use tower::ServiceExt;
use hyper::{Request, StatusCode};

#[tokio::test]
async fn test_create_resource() {
    let ctx = setup().await;
    let client = ApiTestClient::new(ctx.app);
    
    let response = client.post("/api/resources")
        .bearer("mock-token")
        .json(&serde_json::json!({ "name": "Crucible" }))
        .send()
        .await;

    response.assert_status(StatusCode::CREATED);
}
```

---

## 📄 License
MIT — see [LICENSE](../LICENSE) for details.
