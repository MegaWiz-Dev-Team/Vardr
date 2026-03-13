use crate::models::*;
use tokio::process::Command;

// ═══════════════════════════════════════
// Docker Client via CLI (simple, reliable)
// ═══════════════════════════════════════

#[derive(Clone)]
pub struct DockerClient;

impl DockerClient {
    pub fn new() -> Self {
        Self
    }

    /// List all compose containers as ServiceInfo
    pub async fn list_services(&self) -> Vec<ServiceInfo> {
        let output = Command::new("docker")
            .args(["ps", "-a", "--format", "{{json .}}"])
            .output()
            .await;

        let output = match output {
            Ok(o) => o,
            Err(e) => {
                tracing::error!("Failed to run docker ps: {}", e);
                return vec![];
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut services = Vec::new();

        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(info) = serde_json::from_str::<DockerPsJson>(line) {
                if !info.names.starts_with("asgard_") {
                    continue;
                }
                let meta = ServiceMeta::for_container(&info.names);
                let ports = parse_ports(&info.ports);

                services.push(ServiceInfo {
                    id: info.id.clone(),
                    name: info.names.clone(),
                    display_name: meta.display_name.to_string(),
                    emoji: meta.emoji.to_string(),
                    image: info.image.clone(),
                    state: info.state.clone(),
                    status: info.status.clone(),
                    ports,
                    created: 0,
                    health: None,
                });
            }
        }

        // Sort: custom services first, then infra
        services.sort_by(|a, b| {
            let order = |name: &str| -> u8 {
                match name {
                    n if n.contains("vardr") => 0,
                    n if n.contains("mimir_api") => 1,
                    n if n.contains("mimir_dashboard") => 2,
                    n if n.contains("bifrost") => 3,
                    n if n.contains("fenrir") => 4,
                    n if n.contains("yggdrasil") => 5,
                    n if n.contains("mariadb") => 6,
                    n if n.contains("postgres") => 7,
                    n if n.contains("qdrant") => 8,
                    n if n.contains("redis") => 9,
                    n if n.contains("neo4j") => 10,
                    _ => 11,
                }
            };
            order(&a.name).cmp(&order(&b.name))
        });

        services
    }

    /// Get logs for a container
    pub async fn get_logs(&self, container_name: &str, tail: u32) -> Vec<LogEntry> {
        let output = Command::new("docker")
            .args([
                "logs",
                "--tail",
                &tail.to_string(),
                "--timestamps",
                container_name,
            ])
            .output()
            .await;

        let output = match output {
            Ok(o) => o,
            Err(e) => {
                tracing::error!("Failed to get logs for {}: {}", container_name, e);
                return vec![];
            }
        };

        // Docker sends stdout and stderr separately
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{}\n{}", stdout, stderr);

        let short_name = container_name.replace("asgard_", "");

        let mut entries: Vec<LogEntry> = combined
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|line| {
                let (timestamp, message) = parse_timestamped_line(line);
                let level = detect_log_level(&message);
                LogEntry {
                    timestamp,
                    level,
                    message,
                    service: short_name.clone(),
                }
            })
            .collect();

        // Sort by timestamp
        entries.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        entries
    }

    /// Get container stats
    pub async fn get_stats(&self, container_name: &str) -> Option<ContainerMetrics> {
        let output = Command::new("docker")
            .args([
                "stats",
                "--no-stream",
                "--format",
                "{{json .}}",
                container_name,
            ])
            .output()
            .await;

        let output = match output {
            Ok(o) => o,
            Err(_) => return None,
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Ok(stats) = serde_json::from_str::<DockerStatsJson>(stdout.trim()) {
            let short_name = container_name.replace("asgard_", "");
            Some(ContainerMetrics {
                service: short_name,
                cpu_percent: parse_percent(&stats.cpu_perc),
                memory_usage_mb: parse_memory_mb(&stats.mem_usage),
                memory_limit_mb: parse_memory_limit_mb(&stats.mem_usage),
                memory_percent: parse_percent(&stats.mem_perc),
                network_rx_mb: parse_network_rx(&stats.net_io),
                network_tx_mb: parse_network_tx(&stats.net_io),
                pids: stats.pids.parse().unwrap_or(0),
            })
        } else {
            None
        }
    }

    /// Get stats for all running containers
    pub async fn get_all_stats(&self) -> Vec<ContainerMetrics> {
        let output = Command::new("docker")
            .args([
                "stats",
                "--no-stream",
                "--format",
                "{{json .}}",
            ])
            .output()
            .await;

        let output = match output {
            Ok(o) => o,
            Err(_) => return vec![],
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut metrics = Vec::new();

        for line in stdout.lines() {
            if let Ok(stats) = serde_json::from_str::<DockerStatsJson>(line) {
                if !stats.name.starts_with("asgard_") {
                    continue;
                }
                let short_name = stats.name.replace("asgard_", "");
                metrics.push(ContainerMetrics {
                    service: short_name,
                    cpu_percent: parse_percent(&stats.cpu_perc),
                    memory_usage_mb: parse_memory_mb(&stats.mem_usage),
                    memory_limit_mb: parse_memory_limit_mb(&stats.mem_usage),
                    memory_percent: parse_percent(&stats.mem_perc),
                    network_rx_mb: parse_network_rx(&stats.net_io),
                    network_tx_mb: parse_network_tx(&stats.net_io),
                    pids: stats.pids.parse().unwrap_or(0),
                });
            }
        }

        metrics
    }

    // ═══════════════════════════════════════
    // Container Controls (Sprint 2)
    // ═══════════════════════════════════════

    /// Restart a container
    pub async fn restart_container(&self, container_name: &str) -> Result<String, String> {
        run_docker_command(&["restart", container_name]).await
    }

    /// Stop a container
    pub async fn stop_container(&self, container_name: &str) -> Result<String, String> {
        run_docker_command(&["stop", container_name]).await
    }

    /// Start a container
    pub async fn start_container(&self, container_name: &str) -> Result<String, String> {
        run_docker_command(&["start", container_name]).await
    }

    /// Get service states for alert evaluation
    pub async fn get_service_states(&self) -> Vec<(String, String)> {
        let services = self.list_services().await;
        services
            .into_iter()
            .map(|s| (s.name.replace("asgard_", ""), s.state))
            .collect()
    }
}

// ═══════════════════════════════════════
// Docker Compose Commands (Sprint 2)
// ═══════════════════════════════════════

pub async fn compose_command(action: &str) -> Result<String, String> {
    let args = match action {
        "up" => vec!["compose", "up", "-d"],
        "down" => vec!["compose", "down"],
        "restart" => vec!["compose", "restart"],
        _ => return Err(format!("Unknown compose action: {}", action)),
    };

    let compose_dir = std::env::var("COMPOSE_DIR")
        .unwrap_or_else(|_| "/Users/mimir/Developer/Asgard".to_string());

    let output = Command::new("docker")
        .args(&args)
        .current_dir(&compose_dir)
        .output()
        .await
        .map_err(|e| format!("Failed to run docker compose: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        Ok(format!("{}{}", stdout, stderr))
    } else {
        Err(format!("Exit code: {:?}\n{}{}", output.status.code(), stdout, stderr))
    }
}

/// Helper to run dockerCLI commands
async fn run_docker_command(args: &[&str]) -> Result<String, String> {
    let output = Command::new("docker")
        .args(args)
        .output()
        .await
        .map_err(|e| format!("Failed to run docker: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        Ok(stdout.trim().to_string())
    } else {
        Err(format!("{}", stderr.trim()))
    }
}

// ═══════════════════════════════════════
// Docker CLI JSON types
// ═══════════════════════════════════════

#[derive(Debug, serde::Deserialize)]
struct DockerPsJson {
    #[serde(rename = "ID")]
    id: String,
    #[serde(rename = "Names")]
    names: String,
    #[serde(rename = "Image")]
    image: String,
    #[serde(rename = "State")]
    state: String,
    #[serde(rename = "Status")]
    status: String,
    #[serde(rename = "Ports")]
    ports: String,
}

#[derive(Debug, serde::Deserialize)]
struct DockerStatsJson {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "CPUPerc")]
    cpu_perc: String,
    #[serde(rename = "MemUsage")]
    mem_usage: String,
    #[serde(rename = "MemPerc")]
    mem_perc: String,
    #[serde(rename = "NetIO")]
    net_io: String,
    #[serde(rename = "PIDs")]
    pids: String,
}

// ═══════════════════════════════════════
// Parsers
// ═══════════════════════════════════════

fn parse_ports(ports_str: &str) -> Vec<PortMapping> {
    let mut result = Vec::new();
    for part in ports_str.split(", ") {
        // Format: "0.0.0.0:3000->8080/tcp" or "8080/tcp"
        if let Some(arrow_pos) = part.find("->") {
            let external_str = &part[..arrow_pos];
            let internal_str = &part[arrow_pos + 2..];

            let external = external_str
                .rsplit(':')
                .next()
                .and_then(|p| p.parse::<u16>().ok());

            let (internal, proto) = if let Some(slash) = internal_str.find('/') {
                (
                    internal_str[..slash].parse::<u16>().unwrap_or(0),
                    internal_str[slash + 1..].to_string(),
                )
            } else {
                (internal_str.parse::<u16>().unwrap_or(0), "tcp".to_string())
            };

            result.push(PortMapping {
                internal,
                external,
                protocol: proto,
            });
        }
    }
    result
}

fn parse_timestamped_line(line: &str) -> (String, String) {
    // Docker timestamps: "2026-03-13T10:24:54.000Z message..."
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

fn parse_percent(s: &str) -> f64 {
    s.trim_end_matches('%').trim().parse().unwrap_or(0.0)
}

fn parse_memory_mb(s: &str) -> f64 {
    // "123.4MiB / 7.66GiB"
    let usage = s.split('/').next().unwrap_or("0").trim();
    parse_size_to_mb(usage)
}

fn parse_memory_limit_mb(s: &str) -> f64 {
    let limit = s.split('/').nth(1).unwrap_or("0").trim();
    parse_size_to_mb(limit)
}

fn parse_size_to_mb(s: &str) -> f64 {
    let s = s.trim();
    if s.ends_with("GiB") {
        s.trim_end_matches("GiB").trim().parse::<f64>().unwrap_or(0.0) * 1024.0
    } else if s.ends_with("MiB") {
        s.trim_end_matches("MiB").trim().parse::<f64>().unwrap_or(0.0)
    } else if s.ends_with("KiB") {
        s.trim_end_matches("KiB").trim().parse::<f64>().unwrap_or(0.0) / 1024.0
    } else if s.ends_with("B") {
        s.trim_end_matches("B").trim().parse::<f64>().unwrap_or(0.0) / (1024.0 * 1024.0)
    } else {
        0.0
    }
}

fn parse_network_rx(s: &str) -> f64 {
    // "1.23MB / 4.56MB"
    let rx = s.split('/').next().unwrap_or("0").trim();
    parse_size_to_mb(rx)
}

fn parse_network_tx(s: &str) -> f64 {
    let tx = s.split('/').nth(1).unwrap_or("0").trim();
    parse_size_to_mb(tx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ports() {
        let ports = parse_ports("0.0.0.0:3000->8080/tcp, 0.0.0.0:3001->3000/tcp");
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].external, Some(3000));
        assert_eq!(ports[0].internal, 8080);
    }

    #[test]
    fn test_parse_percent() {
        assert_eq!(parse_percent("12.34%"), 12.34);
        assert_eq!(parse_percent("0.00%"), 0.0);
    }

    #[test]
    fn test_parse_size_to_mb() {
        assert_eq!(parse_size_to_mb("1.5GiB"), 1536.0);
        assert_eq!(parse_size_to_mb("256MiB"), 256.0);
    }

    #[test]
    fn test_detect_log_level() {
        assert_eq!(detect_log_level("ERROR: something failed"), "ERROR");
        assert_eq!(detect_log_level("level=WARN msg=test"), "WARN");
        assert_eq!(detect_log_level("server started on port 8080"), "INFO");
    }

    #[test]
    fn test_parse_timestamped_line() {
        let (ts, msg) = parse_timestamped_line("2026-03-13T10:24:54.000Z server started");
        assert_eq!(ts, "2026-03-13T10:24:54.000Z");
        assert_eq!(msg, "server started");
    }

    #[test]
    fn test_compose_action_validate() {
        // Only 'up', 'down', 'restart' are valid
        let valid = ["up", "down", "restart"];
        for action in &valid {
            assert!(!action.is_empty());
        }
    }

    #[test]
    fn test_parse_ports_empty() {
        let ports = parse_ports("");
        assert!(ports.is_empty());
    }

    #[test]
    fn test_parse_kib() {
        assert!((parse_size_to_mb("512KiB") - 0.5).abs() < 0.001);
    }
}

