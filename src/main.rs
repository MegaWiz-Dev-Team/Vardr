mod docker;
mod models;

use axum::{
    Router,
    extract::{Path, Query},
    http::StatusCode,
    response::{Html, IntoResponse, Response, Sse},
    routing::get,
    Json,
};
use axum::response::sse::{Event, KeepAlive};
use docker::DockerClient;
use serde::Deserialize;
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::StreamExt;
use tower_http::cors::CorsLayer;

// ═══════════════════════════════════════
// 🛡️ Várðr — Asgard Monitoring Dashboard
// ═══════════════════════════════════════

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let port = std::env::var("PORT").unwrap_or_else(|_| "9090".to_string());

    let app = Router::new()
        // Pages
        .route("/", get(index_page))
        .route("/health", get(health_check))
        // API
        .route("/api/services", get(api_services))
        .route("/api/services/{name}/logs", get(api_service_logs))
        .route("/api/metrics", get(api_metrics))
        // SSE
        .route("/api/logs/stream/{name}", get(api_log_stream))
        // Static
        .route("/style.css", get(css_file))
        .route("/app.js", get(js_file))
        .layer(CorsLayer::permissive());

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("🛡️ Várðr listening on http://{}", addr);

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
// API Endpoints
// ═══════════════════════════════════════

async fn api_services() -> Json<Vec<models::ServiceInfo>> {
    let client = DockerClient::new();
    let services = client.list_services().await;
    Json(services)
}

#[derive(Deserialize)]
struct LogQuery {
    tail: Option<u32>,
    level: Option<String>,
    search: Option<String>,
}

async fn api_service_logs(
    Path(name): Path<String>,
    Query(params): Query<LogQuery>,
) -> Json<Vec<models::LogEntry>> {
    let client = DockerClient::new();
    let container_name = if name.starts_with("asgard_") {
        name.clone()
    } else {
        format!("asgard_{}", name)
    };

    let tail = params.tail.unwrap_or(100);
    let mut logs = client.get_logs(&container_name, tail).await;

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

async fn api_metrics() -> Json<Vec<models::ContainerMetrics>> {
    let client = DockerClient::new();
    let metrics = client.get_all_stats().await;
    Json(metrics)
}

// ═══════════════════════════════════════
// SSE Log Stream
// ═══════════════════════════════════════

async fn api_log_stream(
    Path(name): Path<String>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let container_name = if name.starts_with("asgard_") {
        name.clone()
    } else {
        format!("asgard_{}", name)
    };

    let stream = async_stream::stream! {
        let client = DockerClient::new();
        let mut last_count = 0usize;

        loop {
            let logs = client.get_logs(&container_name, 50).await;
            let current_count = logs.len();

            if current_count != last_count && !logs.is_empty() {
                // Send only new entries
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
