use axum::{
    extract::Query,
    response::{sse::{Event, KeepAlive, Sse}, Json},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, net::SocketAddr, path::PathBuf, process::Stdio};
use sysinfo::System;
use tokio::{io::{AsyncBufReadExt, BufReader}, process::Command, sync::Mutex};
use tower_http::cors::CorsLayer;
use tracing_subscriber;

// ═══════════════════════════════════════
// Models
// ═══════════════════════════════════════

#[derive(Serialize)]
pub struct ContainerMetrics {
    pub service: String,
    pub cpu_percent: f64,
    pub memory_usage_mb: f64,
    pub memory_limit_mb: f64,
    pub memory_percent: f64,
    pub network_rx_mb: f64,
    pub network_tx_mb: f64,
    pub pids: u64,
}

#[derive(Serialize)]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub name: String,
    pub memory_mb: f64,
    pub cpu_percent: f64,
}

#[derive(Serialize)]
pub struct SystemMetrics {
    /// Apple Silicon uses unified memory — physical RAM doubles as VRAM.
    /// `memory_total_mb` therefore covers both. There is no separate VRAM
    /// figure to report; a dedicated GPU box would need `powermetrics`
    /// (root privileges) which is out of scope for the user-mode agent.
    pub apple_silicon_unified_memory: bool,
    pub memory_total_mb: f64,
    pub memory_used_mb: f64,
    pub memory_available_mb: f64,
    pub memory_used_percent: f64,
    pub swap_total_mb: f64,
    pub swap_used_mb: f64,
    pub cpu_count: usize,
    pub cpu_percent_global: f64,
    pub load_avg_1m: f64,
    pub load_avg_5m: f64,
    pub load_avg_15m: f64,
    pub uptime_seconds: u64,
    pub top_memory_processes: Vec<ProcessSnapshot>,
}

#[derive(Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
    pub service: String,
}

#[derive(Deserialize)]
struct LogQuery {
    tail: Option<u32>,
    level: Option<String>,
    search: Option<String>,
}

// ═══════════════════════════════════════
// State
// ═══════════════════════════════════════

struct AppState {
    sys: Mutex<System>,
}

// ═══════════════════════════════════════
// App
// ═══════════════════════════════════════

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let state = std::sync::Arc::new(AppState {
        sys: Mutex::new(System::new_all()),
    });

    let app = Router::new()
        .route("/health", get(|| async { Json(serde_json::json!({"status": "ok"})) }))
        .route("/api/metrics", get(api_metrics))
        .route("/api/system", get(api_system))
        .route("/api/logs", get(api_logs))
        .route("/api/logs/stream", get(api_logs_stream))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port = 9091; // Native Agent listens on 9091
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("🛡️ Várðr Native Agent listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ═══════════════════════════════════════
// Handlers
// ═══════════════════════════════════════

async fn api_metrics(
    axum::extract::State(state): axum::extract::State<std::sync::Arc<AppState>>,
) -> Json<Vec<ContainerMetrics>> {
    let mut sys = state.sys.lock().await;
    sys.refresh_all();
    
    let process_name = "heimdall-gateway";
    let mut total_cpu = 0.0;
    let mut total_mem_mb = 0.0;
    let mut pids = 0;

    for (_pid, process) in sys.processes() {
        if process.name().to_string_lossy().to_lowercase().contains(process_name) {
            total_cpu += process.cpu_usage() as f64;
            // sysinfo returns bytes. Convert to MB
            total_mem_mb += (process.memory() as f64) / 1024.0 / 1024.0;
            pids += 1;
        }
    }

    let total_sys_mem_mb = (sys.total_memory() as f64) / 1024.0 / 1024.0;
    let mem_percent = if total_sys_mem_mb > 0.0 {
        (total_mem_mb / total_sys_mem_mb) * 100.0
    } else {
        0.0
    };

    let metrics = vec![ContainerMetrics {
        service: "heimdall_gateway".to_string(), // Matches Várðr UI mapping
        cpu_percent: total_cpu,
        memory_usage_mb: total_mem_mb,
        memory_limit_mb: total_sys_mem_mb,
        memory_percent: mem_percent,
        network_rx_mb: 0.0,
        network_tx_mb: 0.0,
        pids,
    }];

    Json(metrics)
}

async fn api_system(
    axum::extract::State(state): axum::extract::State<std::sync::Arc<AppState>>,
) -> Json<SystemMetrics> {
    let mut sys = state.sys.lock().await;
    sys.refresh_all();

    let total_mb = (sys.total_memory() as f64) / 1024.0 / 1024.0;
    let used_mb = (sys.used_memory() as f64) / 1024.0 / 1024.0;
    let available_mb = (sys.available_memory() as f64) / 1024.0 / 1024.0;
    let used_pct = if total_mb > 0.0 { (used_mb / total_mb) * 100.0 } else { 0.0 };

    let swap_total = (sys.total_swap() as f64) / 1024.0 / 1024.0;
    let swap_used = (sys.used_swap() as f64) / 1024.0 / 1024.0;

    let global_cpu = sys.global_cpu_usage() as f64;
    let load = System::load_average();
    let uptime = System::uptime();
    let cpu_count = sys.cpus().len();

    // Top 8 processes by RSS (gemma bench, qwen, etc. land here during ML work).
    let mut procs: Vec<ProcessSnapshot> = sys.processes().iter()
        .map(|(pid, p)| ProcessSnapshot {
            pid: pid.as_u32(),
            name: p.name().to_string_lossy().to_string(),
            memory_mb: (p.memory() as f64) / 1024.0 / 1024.0,
            cpu_percent: p.cpu_usage() as f64,
        })
        .collect();
    procs.sort_by(|a, b| b.memory_mb.partial_cmp(&a.memory_mb).unwrap_or(std::cmp::Ordering::Equal));
    procs.truncate(8);

    Json(SystemMetrics {
        apple_silicon_unified_memory: cfg!(target_arch = "aarch64") && cfg!(target_os = "macos"),
        memory_total_mb: total_mb,
        memory_used_mb: used_mb,
        memory_available_mb: available_mb,
        memory_used_percent: used_pct,
        swap_total_mb: swap_total,
        swap_used_mb: swap_used,
        cpu_count,
        cpu_percent_global: global_cpu,
        load_avg_1m: load.one,
        load_avg_5m: load.five,
        load_avg_15m: load.fifteen,
        uptime_seconds: uptime,
        top_memory_processes: procs,
    })
}

fn get_log_file_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/mimir".into());
    PathBuf::from(home).join("Developer/Heimdall/logs/gateway-stdout.log")
}

fn parse_timestamped_line(line: &str) -> (String, String) {
    if line.len() > 30 && (line.contains('T') && (line.contains('Z') || line.contains('+'))) {
        if let Some(space) = line.find(' ') {
            if space < 40 {
                return (line[..space].to_string(), line[space + 1..].to_string());
            }
        }
    }
    (
        chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        line.to_string(),
    )
}

fn detect_log_level(msg: &str) -> String {
    let upper = msg.to_uppercase();
    if upper.contains("ERROR") || upper.contains("FATAL") || upper.contains("PANIC") {
        "ERROR".to_string()
    } else if upper.contains("WARN") {
        "WARN".to_string()
    } else if upper.contains("DEBUG") || upper.contains("TRACE") {
        "DEBUG".to_string()
    } else {
        "INFO".to_string()
    }
}

async fn api_logs(Query(params): Query<LogQuery>) -> Json<Vec<LogEntry>> {
    let tail = params.tail.unwrap_or(100);
    let log_file = get_log_file_path();
    
    let output = Command::new("tail")
        .args(["-n", &tail.to_string(), log_file.to_str().unwrap()])
        .output()
        .await;

    let mut entries = Vec::new();
    
    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.trim().is_empty() { continue; }
            let (timestamp, message) = parse_timestamped_line(line);
            let level = detect_log_level(&message);
            entries.push(LogEntry {
                timestamp,
                level,
                message,
                service: "heimdall_gateway".to_string(),
            });
        }
    }

    if let Some(level) = &params.level {
        if !level.is_empty() && level != "ALL" {
            let level_upper = level.to_uppercase();
            entries.retain(|l| l.level == level_upper);
        }
    }
    if let Some(search) = &params.search {
        if !search.is_empty() {
            let search_lower = search.to_lowercase();
            entries.retain(|l| l.message.to_lowercase().contains(&search_lower));
        }
    }

    Json(entries)
}

async fn api_logs_stream() -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let log_file = get_log_file_path();
    
    let stream = async_stream::stream! {
        let mut child = Command::new("tail")
            .args(["-n", "50", "-F", log_file.to_str().unwrap()])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to run tail -F");

        let stdout = child.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout).lines();

        while let Ok(Some(line)) = reader.next_line().await {
            if line.trim().is_empty() { continue; }
            
            let (timestamp, message) = parse_timestamped_line(&line);
            let level = detect_log_level(&message);
            
            let entry = LogEntry {
                timestamp,
                level,
                message,
                service: "heimdall_gateway".to_string(),
            };

            if let Ok(json) = serde_json::to_string(&entry) {
                yield Ok(Event::default().data(json));
            }
        }
        
        let _ = child.kill().await;
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}
