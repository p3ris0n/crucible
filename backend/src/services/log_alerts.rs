use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;
use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct LogAlertRule {
    pub id: Uuid,
    pub name: String,
    pub pattern: String,
    pub threshold: i32,
    pub interval_seconds: i32,
    pub is_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRuleRequest {
    pub name: String,
    pub pattern: String,
    pub threshold: i32,
    pub interval_seconds: i32,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct LogAlert {
    pub id: Uuid,
    pub rule_id: Uuid,
    pub message: String,
    pub triggered_at: chrono::DateTime<chrono::Utc>,
}

pub struct ServiceState {
    pub db: PgPool,
    pub redis: redis::Client,
}

pub fn router() -> Router {
    Router::new()
        .route("/rules", post(create_rule).get(list_rules))
        .route("/rules/:id", get(get_rule))
        .route("/ingest", post(ingest_log))
}

async fn create_rule(
    State(state): State<Arc<ServiceState>>,
    Json(payload): Json<CreateRuleRequest>,
) -> Result<Json<LogAlertRule>, AppError> {
    let rule = sqlx::query_as::<_, LogAlertRule>(
        "INSERT INTO log_alert_rules (name, pattern, threshold, interval_seconds) 
         VALUES ($1, $2, $3, $4) RETURNING *"
    )
    .bind(payload.name)
    .bind(payload.pattern)
    .bind(payload.threshold)
    .bind(payload.interval_seconds)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(rule))
}

async fn list_rules(
    State(state): State<Arc<ServiceState>>,
) -> Result<Json<Vec<LogAlertRule>>, AppError> {
    let rules = sqlx::query_as::<_, LogAlertRule>("SELECT * FROM log_alert_rules")
        .fetch_all(&state.db)
        .await?;
    Ok(Json(rules))
}

async fn get_rule(
    State(state): State<Arc<ServiceState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<LogAlertRule>, AppError> {
    let rule = sqlx::query_as::<_, LogAlertRule>("SELECT * FROM log_alert_rules WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Rule not found: {}", id)))?;
    
    Ok(Json(rule))
}

#[derive(Debug, Deserialize)]
pub struct LogEntry {
    pub message: String,
    pub level: String,
}

async fn ingest_log(
    State(state): State<Arc<ServiceState>>,
    Json(log): Json<LogEntry>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!("Processing log: {}", log.message);
    
    // 1. Fetch all enabled rules
    let rules = sqlx::query_as::<_, LogAlertRule>(
        "SELECT * FROM log_alert_rules WHERE is_enabled = true"
    )
    .fetch_all(&state.db)
    .await?;

    let mut matched_rules = Vec::new();

    for rule in rules {
        if log.message.contains(&rule.pattern) {
            tracing::debug!("Log matched pattern for rule: {}", rule.name);
            
            // 2. Increment count in Redis with TTL
            let redis_key = format!("alert_count:{}:{}", rule.id, chrono::Utc::now().timestamp() / rule.interval_seconds as i64);
            let mut conn = state.redis.get_async_connection().await?;
            
            let count: i32 = redis::cmd("INCR")
                .arg(&redis_key)
                .query_async(&mut conn)
                .await?;
            
            // Set TTL if new key
            if count == 1 {
                let _: () = redis::cmd("EXPIRE")
                    .arg(&redis_key)
                    .arg(rule.interval_seconds)
                    .query_async(&mut conn)
                    .await?;
            }

            // 3. Check if threshold reached
            if count >= rule.threshold {
                tracing::warn!("Threshold reached for rule: {}. Triggering alert!", rule.name);
                
                // 4. Persist alert
                sqlx::query(
                    "INSERT INTO log_alerts (rule_id, message) VALUES ($1, $2)"
                )
                .bind(rule.id)
                .bind(format!("Threshold of {} reached for pattern '{}'", rule.threshold, rule.pattern))
                .execute(&state.db)
                .await?;
                
                matched_rules.push(rule.name);
            }
        }
    }
    
    Ok(Json(serde_json::json!({ 
        "status": "processed",
        "matched": matched_rules
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matching() {
        let pattern = "error";
        let message = "This is an error message";
        assert!(message.contains(pattern));
    }
}
