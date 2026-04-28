# Crucible Backend

High-performance Rust backend for Log-Based Alerting.

## Technical Stack
- **Axum**: High-performance web framework.
- **SQLx**: Async PostgreSQL driver with compile-time checked queries.
- **Redis**: Caching and threshold tracking.
- **Tracing**: Observability and structured logging.

## API Endpoints

### Rules Management
- `GET /api/alerts/rules` - List all alerting rules.
- `POST /api/alerts/rules` - Create a new alerting rule.
- `GET /api/alerts/rules/:id` - Get details of a specific rule.

### Log Ingestion
- `POST /api/alerts/ingest` - Ingest a log entry for pattern matching.

## Database Schema
```sql
CREATE TABLE log_alert_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    pattern TEXT NOT NULL,
    threshold INT NOT NULL DEFAULT 1,
    interval_seconds INT NOT NULL DEFAULT 60,
    is_enabled BOOLEAN NOT NULL DEFAULT true
);

CREATE TABLE log_alerts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rule_id UUID NOT NULL REFERENCES log_alert_rules(id),
    message TEXT NOT NULL,
    triggered_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```
