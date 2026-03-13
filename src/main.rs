mod alerts;
mod docker;
mod models;

use alerts::AlertEngine;
use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response, Sse},
    routing::{get, post},
    Json,
};
use axum::response::sse::{Event, KeepAlive};
use docker::DockerClient;
use serde::Deserialize;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::CorsLayer;

// ═══════════════════════════════════════
// 🛡️ Várðr — Asgard Monitoring Dashboard
// ═══════════════════════════════════════

#[derive(Clone)]
struct AppState {
    docker: DockerClient,
    alerts: Arc<AlertEngine>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let port = std::env::var("PORT").unwrap_or_else(|_| "9090".to_string());

    let state = AppState {
        docker: DockerClient::new(),
        alerts: Arc::new(AlertEngine::new()),
    };

    // Background alert evaluator
    let bg_state = state.clone();
    tokio::spawn(async move {
        loop {
            let metrics = bg_state.docker.get_all_stats().await;
            let states = bg_state.docker.get_service_states().await;
            bg_state.alerts.evaluate(&metrics, &states).await;
            tokio::time::sleep(Duration::from_secs(15)).await;
        }
    });

    let app = Router::new()
        // Pages
        .route("/", get(index_page))
        .route("/health", get(health_check))
        // API — Services (Sprint 1)
        .route("/api/services", get(api_services))
        .route("/api/services/{name}/logs", get(api_service_logs))
        .route("/api/metrics", get(api_metrics))
        // SSE
        .route("/api/logs/stream/{name}", get(api_log_stream))
        // API — Container Controls (Sprint 2)
        .route("/api/containers/{name}/restart", post(api_container_restart))
        .route("/api/containers/{name}/stop", post(api_container_stop))
        .route("/api/containers/{name}/start", post(api_container_start))
        // API — Docker Compose (Sprint 2)
        .route("/api/compose/{action}", post(api_compose))
        // API — Alerts (Sprint 2)
        .route("/api/alerts", get(api_alerts_list))
        .route("/api/alerts/summary", get(api_alerts_summary))
        .route("/api/alerts/rules", get(api_alerts_rules))
        .route("/api/alerts/rules", post(api_alerts_add_rule))
        // Static
        .route("/style.css", get(css_file))
        .route("/app.js", get(js_file))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("🛡️ Várðr v{} listening on http://{}", env!("CARGO_PKG_VERSION"), addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ═══════════════════════════════════════
// Pages
// ═══════════════════════════════════════

async fn index_page() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

async fn css_file() -> Response {
    (
        StatusCode::OK,
        [("content-type", "text/css")],
        include_str!("../static/style.css"),
    ).into_response()
}

async fn js_file() -> Response {
    (
        StatusCode::OK,
        [("content-type", "application/javascript")],
        include_str!("../static/app.js"),
    ).into_response()
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "vardr",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

// ═══════════════════════════════════════
// API — Services (Sprint 1)
// ═══════════════════════════════════════

async fn api_services(State(state): State<AppState>) -> Json<Vec<models::ServiceInfo>> {
    let services = state.docker.list_services().await;
    Json(services)
}

#[derive(Deserialize)]
struct LogQuery {
    tail: Option<u32>,
    level: Option<String>,
    search: Option<String>,
}

async fn api_service_logs(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(params): Query<LogQuery>,
) -> Json<Vec<models::LogEntry>> {
    let container_name = resolve_container_name(&name);
    let tail = params.tail.unwrap_or(100);
    let mut logs = state.docker.get_logs(&container_name, tail).await;

    // Filter by level
    if let Some(level) = &params.level {
        if !level.is_empty() && level != "ALL" {
            let level_upper = level.to_uppercase();
            logs.retain(|l| l.level == level_upper);
        }
    }

    // Filter by search keyword
    if let Some(search) = &params.search {
        if !search.is_empty() {
            let search_lower = search.to_lowercase();
            logs.retain(|l| l.message.to_lowercase().contains(&search_lower));
        }
    }

    Json(logs)
}

async fn api_metrics(State(state): State<AppState>) -> Json<Vec<models::ContainerMetrics>> {
    let metrics = state.docker.get_all_stats().await;
    Json(metrics)
}

// ═══════════════════════════════════════
// SSE Log Stream
// ═══════════════════════════════════════

async fn api_log_stream(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let container_name = resolve_container_name(&name);

    let stream = async_stream::stream! {
        let mut last_count = 0usize;

        loop {
            let logs = state.docker.get_logs(&container_name, 50).await;
            let current_count = logs.len();

            if current_count != last_count && !logs.is_empty() {
                let new_entries = if last_count < current_count {
                    &logs[last_count..]
                } else {
                    &logs[..]
                };

                for entry in new_entries {
                    if let Ok(json) = serde_json::to_string(entry) {
                        yield Ok(Event::default().data(json));
                    }
                }
                last_count = current_count;
            }

            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

// ═══════════════════════════════════════
// API — Container Controls (Sprint 2)
// ═══════════════════════════════════════

async fn api_container_restart(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Response {
    let container_name = resolve_container_name(&name);
    tracing::info!("🔄 Restarting container: {}", container_name);
    match state.docker.restart_container(&container_name).await {
        Ok(msg) => (StatusCode::OK, Json(serde_json::json!({
            "status": "ok", "action": "restart", "container": container_name, "message": msg
        }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
            "status": "error", "message": e
        }))).into_response(),
    }
}

async fn api_container_stop(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Response {
    let container_name = resolve_container_name(&name);
    tracing::info!("⏹ Stopping container: {}", container_name);
    match state.docker.stop_container(&container_name).await {
        Ok(msg) => (StatusCode::OK, Json(serde_json::json!({
            "status": "ok", "action": "stop", "container": container_name, "message": msg
        }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
            "status": "error", "message": e
        }))).into_response(),
    }
}

async fn api_container_start(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Response {
    let container_name = resolve_container_name(&name);
    tracing::info!("▶ Starting container: {}", container_name);
    match state.docker.start_container(&container_name).await {
        Ok(msg) => (StatusCode::OK, Json(serde_json::json!({
            "status": "ok", "action": "start", "container": container_name, "message": msg
        }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
            "status": "error", "message": e
        }))).into_response(),
    }
}

// ═══════════════════════════════════════
// API — Docker Compose (Sprint 2)
// ═══════════════════════════════════════

async fn api_compose(Path(action): Path<String>) -> Response {
    tracing::info!("🐳 Docker Compose: {}", action);
    match docker::compose_command(&action).await {
        Ok(msg) => (StatusCode::OK, Json(serde_json::json!({
            "status": "ok", "action": action, "output": msg
        }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
            "status": "error", "action": action, "message": e
        }))).into_response(),
    }
}

// ═══════════════════════════════════════
// API — Alerts (Sprint 2)
// ═══════════════════════════════════════

async fn api_alerts_list(State(state): State<AppState>) -> Json<Vec<alerts::Alert>> {
    let active = state.alerts.active_alerts.read().await;
    Json(active.clone())
}

async fn api_alerts_summary(State(state): State<AppState>) -> Json<alerts::AlertSummary> {
    let summary = state.alerts.summary().await;
    Json(summary)
}

async fn api_alerts_rules(State(state): State<AppState>) -> Json<Vec<alerts::AlertRule>> {
    let rules = state.alerts.rules.read().await;
    Json(rules.clone())
}

async fn api_alerts_add_rule(
    State(state): State<AppState>,
    Json(rule): Json<alerts::AlertRule>,
) -> Response {
    let mut rules = state.alerts.rules.write().await;
    tracing::info!("➕ Adding alert rule: {} ({})", rule.name, rule.id);
    rules.push(rule);
    (StatusCode::CREATED, Json(serde_json::json!({
        "status": "ok", "total_rules": rules.len()
    }))).into_response()
}

// ═══════════════════════════════════════
// Helpers
// ═══════════════════════════════════════

fn resolve_container_name(name: &str) -> String {
    if name.starts_with("asgard_") {
        name.to_string()
    } else {
        format!("asgard_{}", name)
    }
}
