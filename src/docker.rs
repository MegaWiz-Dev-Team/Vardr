use crate::models::*;
use reqwest::Client;
use serde_json::Value;

// ═══════════════════════════════════════
// Kubernetes Native Client Wrapper (Replaces Docker)
// ═══════════════════════════════════════

#[derive(Clone)]
pub struct DockerClient {
    client: Client,
    token: String,
    api_url: String,
}

impl DockerClient {
    pub fn new() -> Self {
        let token = std::fs::read_to_string("/var/run/secrets/kubernetes.io/serviceaccount/token")
            .unwrap_or_default();
        let api_url = "https://kubernetes.default.svc/api/v1/namespaces/asgard".to_string();
        
        let client = Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap_or(Client::new());

        Self { client, token, api_url }
    }

    /// Helper for authorization header
    fn get_auth(&self) -> String {
        format!("Bearer {}", self.token)
    }

    /// List all K3s Pods as ServiceInfo
    pub async fn list_services(&self) -> Vec<ServiceInfo> {
        let url = format!("{}/pods", self.api_url);
        let res = self.client.get(&url).header("Authorization", self.get_auth()).send().await;
        
        let mut services = Vec::new();

        if let Ok(response) = res {
            if let Ok(json) = response.json::<Value>().await {
                if let Some(items) = json["items"].as_array() {
                    for item in items {
                        let name = item["metadata"]["name"].as_str().unwrap_or("unknown");
                        let mut base_name = name;
                        
                        // Try to cleanly get the app label
                        if let Some(labels) = item["metadata"].get("labels") {
                            if let Some(app_label) = labels.get("app") {
                                base_name = app_label.as_str().unwrap_or(base_name);
                            }
                        }

                        let state = item["status"]["phase"].as_str().unwrap_or("Unknown");
                        
                        let meta = ServiceMeta::for_container(base_name);

                        services.push(ServiceInfo {
                            id: item["metadata"]["uid"].as_str().unwrap_or("").to_string(),
                            name: name.to_string(), // use the pod name natively
                            display_name: meta.display_name.to_string(),
                            emoji: meta.emoji.to_string(),
                            image: "k3s native".into(),
                            state: state.to_lowercase(),
                            status: state.to_string(),
                            ports: vec![],
                            created: 0,
                            health: None,
                        });
                    }
                } else {
                    tracing::error!("K8s API returned no items array");
                }
            } else {
                tracing::error!("K8s API failed to parse json");
            }
        } else {
            tracing::error!("K8s API request failed");
        }

        // --- Native Agent Injection (Heimdall) ---
        let agent_url = std::env::var("VARDR_AGENT_URL").unwrap_or_else(|_| "http://host.k3d.internal:9091".to_string());
        if let Ok(agent_health) = self.client.get(format!("{}/health", agent_url)).timeout(std::time::Duration::from_secs(2)).send().await {
            let meta = ServiceMeta::for_container("heimdall_gateway");
            services.push(ServiceInfo {
                id: "launchd_heimdall".into(),
                name: "asgard_heimdall_gateway".into(),
                display_name: meta.display_name.to_string(),
                emoji: meta.emoji.to_string(),
                image: "bare-metal macOS".into(),
                state: "running".into(),
                status: "Up (Native macOS)".into(),
                ports: vec![PortMapping { internal: 8080, external: Some(8080), protocol: "tcp".into() }],
                created: 0,
                health: None,
            });
        }

        // Sort: custom services first, then infra
        services.sort_by(|a, b| {
            let order = |name: &str| -> u8 {
                match name {
                    n if n.contains("vardr") => 0,
                    n if n.contains("mimir_api") || n.contains("mimir-api") => 1,
                    n if n.contains("mimir_dashboard") || n.contains("mimir-dashboard") => 2,
                    n if n.contains("bifrost") => 3,
                    n if n.contains("fenrir") => 4,
                    n if n.contains("yggdrasil") => 5,
                    n if n.contains("portal") => 6,
                    n if n.contains("vault") || n.contains("fafnir") => 7,
                    n if n.contains("forseti") => 8,
                    _ => 13,
                }
            };
            order(&a.name).cmp(&order(&b.name))
        });

        services
    }

    /// Get logs for a pod
    pub async fn get_logs(&self, container_name: &str, tail: u32) -> Vec<LogEntry> {
        if container_name == "asgard_heimdall_gateway" || container_name == "heimdall_gateway" {
            let agent_url = std::env::var("VARDR_AGENT_URL").unwrap_or_else(|_| "http://host.k3d.internal:9091".to_string());
            if let Ok(res) = self.client.get(format!("{}/api/logs?tail={}", agent_url, tail)).timeout(std::time::Duration::from_secs(5)).send().await {
                if let Ok(agent_logs) = res.json::<Vec<LogEntry>>().await {
                    return agent_logs;
                }
            }
            return vec![];
        }

        let url = format!("{}/pods/{}/log?tailLines={}", self.api_url, container_name, tail);
        let res = self.client.get(&url).header("Authorization", self.get_auth()).send().await;

        if let Ok(response) = res {
            if let Ok(text) = response.text().await {
                return text.lines().map(|line| {
                    LogEntry {
                        timestamp: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                        level: detect_log_level(line),
                        message: line.to_string(),
                        service: container_name.to_string(),
                    }
                }).collect();
            }
        }
        vec![]
    }

    /// Get stats (Fallback)
    pub async fn get_stats(&self, container_name: &str) -> Option<ContainerMetrics> {
        Some(ContainerMetrics {
            service: container_name.to_string(),
            cpu_percent: 0.0,
            memory_usage_mb: 0.0,
            memory_limit_mb: 0.0,
            memory_percent: 0.0,
            network_rx_mb: 0.0,
            network_tx_mb: 0.0,
            pids: 0,
        })
    }

    /// Get stats for all pods
    pub async fn get_all_stats(&self) -> Vec<ContainerMetrics> {
        let services = self.list_services().await;
        services.into_iter().map(|s| ContainerMetrics {
            service: s.name,
            cpu_percent: 0.0,
            memory_usage_mb: 0.0,
            memory_limit_mb: 0.0,
            memory_percent: 0.0,
            network_rx_mb: 0.0,
            network_tx_mb: 0.0,
            pids: 0,
        }).collect()
    }

    // ═══════════════════════════════════════
    // Container Controls
    // ═══════════════════════════════════════

    pub async fn restart_container(&self, container_name: &str) -> Result<String, String> {
        let url = format!("{}/pods/{}", self.api_url, container_name);
        match self.client.delete(&url).header("Authorization", self.get_auth()).send().await {
            Ok(_) => Ok("Pod deleted, Deployment will recreate it".into()),
            Err(e) => Err(e.to_string()),
        }
    }

    pub async fn stop_container(&self, _container_name: &str) -> Result<String, String> {
        Ok("Scaling down not supported in simple proxy".into())
    }

    pub async fn start_container(&self, _container_name: &str) -> Result<String, String> {
        Ok("Already running".into())
    }

    pub async fn get_service_states(&self) -> Vec<(String, String)> {
        let services = self.list_services().await;
        services.into_iter().map(|s| (s.name, s.state)).collect()
    }
}

pub async fn compose_command(action: &str) -> Result<String, String> {
    Ok(format!("Kubernetes Native Mode Active: Ignored compose command '{}'", action))
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
