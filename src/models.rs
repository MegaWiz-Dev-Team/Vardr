use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════
// Docker Container Info
// ═══════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerContainer {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Names")]
    pub names: Vec<String>,
    #[serde(rename = "Image")]
    pub image: String,
    #[serde(rename = "State")]
    pub state: String,
    #[serde(rename = "Status")]
    pub status: String,
    #[serde(rename = "Ports")]
    pub ports: Vec<DockerPort>,
    #[serde(rename = "Created")]
    pub created: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerPort {
    #[serde(rename = "PrivatePort")]
    pub private_port: u16,
    #[serde(rename = "PublicPort", default)]
    pub public_port: Option<u16>,
    #[serde(rename = "Type")]
    pub port_type: String,
}

// ═══════════════════════════════════════
// Service Info (our API response)
// ═══════════════════════════════════════

#[derive(Debug, Clone, Serialize)]
pub struct ServiceInfo {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub emoji: String,
    pub image: String,
    pub state: String,
    pub status: String,
    pub ports: Vec<PortMapping>,
    pub created: i64,
    pub health: Option<HealthStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortMapping {
    pub internal: u16,
    pub external: Option<u16>,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub response_time_ms: u64,
    pub last_check: String,
    pub endpoint: String,
}

// ═══════════════════════════════════════
// Log Entry
// ═══════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
    pub service: String,
}

// ═══════════════════════════════════════
// Container Metrics
// ═══════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

// ═══════════════════════════════════════
// Docker Stats (raw from API)
// ═══════════════════════════════════════

#[derive(Debug, Clone, Deserialize)]
pub struct DockerStats {
    pub cpu_stats: CpuStats,
    pub precpu_stats: CpuStats,
    pub memory_stats: MemoryStats,
    #[serde(default)]
    pub networks: Option<std::collections::HashMap<String, NetworkStats>>,
    #[serde(default)]
    pub pids_stats: PidsStats,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CpuStats {
    pub cpu_usage: CpuUsage,
    #[serde(default)]
    pub system_cpu_usage: Option<u64>,
    #[serde(default)]
    pub online_cpus: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CpuUsage {
    pub total_usage: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MemoryStats {
    #[serde(default)]
    pub usage: Option<u64>,
    #[serde(default)]
    pub limit: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NetworkStats {
    #[serde(default)]
    pub rx_bytes: u64,
    #[serde(default)]
    pub tx_bytes: u64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PidsStats {
    #[serde(default)]
    pub current: Option<u64>,
}

// ═══════════════════════════════════════
// Service Metadata (display names, emojis)
// ═══════════════════════════════════════

pub struct ServiceMeta {
    pub display_name: &'static str,
    pub emoji: &'static str,
    pub health_endpoint: Option<&'static str>,
    pub health_port: Option<u16>,
}

impl ServiceMeta {
    pub fn for_container(name: &str) -> Self {
        match name {
            n if n.contains("mimir_api") || n.contains("mimir-api") => Self {
                display_name: "Mimir API",
                emoji: "🧠",
                health_endpoint: Some("/health"),
                health_port: Some(3000),
            },
            n if n.contains("mimir_dashboard") || n.contains("mimir-dashboard") => Self {
                display_name: "Mimir Dashboard",
                emoji: "🖥️",
                health_endpoint: None,
                health_port: Some(3001),
            },
            n if n.contains("bifrost") => Self {
                display_name: "Bifrost",
                emoji: "⚡",
                health_endpoint: Some("/healthz"),
                health_port: Some(8100),
            },
            n if n.contains("fenrir") => Self {
                display_name: "Fenrir",
                emoji: "🐺",
                health_endpoint: Some("/health"),
                health_port: Some(8200),
            },
            n if n.contains("yggdrasil") => Self {
                display_name: "Yggdrasil",
                emoji: "🌳",
                health_endpoint: None,
                health_port: Some(8085),
            },
            n if n.contains("vardr") => Self {
                display_name: "Várðr",
                emoji: "🛡️",
                health_endpoint: Some("/health"),
                health_port: Some(9090),
            },
            n if n.contains("mariadb") => Self {
                display_name: "MariaDB",
                emoji: "🗄️",
                health_endpoint: None,
                health_port: None,
            },
            n if n.contains("postgres") => Self {
                display_name: "PostgreSQL",
                emoji: "🐘",
                health_endpoint: None,
                health_port: None,
            },
            n if n.contains("qdrant") => Self {
                display_name: "Qdrant",
                emoji: "🔍",
                health_endpoint: None,
                health_port: Some(6333),
            },
            n if n.contains("redis") => Self {
                display_name: "Redis",
                emoji: "📦",
                health_endpoint: None,
                health_port: None,
            },
            n if n.contains("neo4j") => Self {
                display_name: "Neo4j",
                emoji: "🕸️",
                health_endpoint: None,
                health_port: Some(7474),
            },
            n if n.contains("portal") => Self {
                display_name: "Asgard Portal",
                emoji: "🚀",
                health_endpoint: Some("/health"),
                health_port: Some(3000),
            },
            n if n.contains("llmgoat") || n.contains("goat") => Self {
                display_name: "LLM Goat",
                emoji: "🐐",
                health_endpoint: None,
                health_port: Some(8080),
            },
            n if n.contains("mjolnir") => Self {
                display_name: "Mjolnir",
                emoji: "🔨",
                health_endpoint: None,
                health_port: None,
            },
            n if n.contains("hermodr-eir") || n.contains("hermodr_eir") => Self {
                display_name: "Hermóðr (Eir WebView)",
                emoji: "📱",
                health_endpoint: None,
                health_port: None,
            },
            n if n.contains("hermodr") => Self {
                display_name: "Hermóðr",
                emoji: "💬",
                health_endpoint: None,
                health_port: Some(3000),
            },
            n if n.contains("eir_gateway") || n.contains("eir-gateway") => Self {
                display_name: "Eir Gateway",
                emoji: "🏥",
                health_endpoint: Some("/healthz"),
                health_port: Some(8300),
            },
            n if n.contains("eir") => Self {
                display_name: "Eir (OpenEMR)",
                emoji: "💊",
                health_endpoint: None,
                health_port: Some(80),
            },
            n if n.contains("forseti") => Self {
                display_name: "Forseti",
                emoji: "⚖️",
                health_endpoint: Some("/"),
                health_port: Some(5555),
            },
            n if n.contains("ratatoskr") => Self {
                display_name: "Ratatoskr",
                emoji: "🐿️",
                health_endpoint: Some("/health"),
                health_port: Some(9200),
            },
            n if n.contains("pageindex") => Self {
                display_name: "PageIndex",
                emoji: "📑",
                health_endpoint: Some("/health"),
                health_port: Some(8600),
            },
            n if n.contains("fafnir") || n.contains("vault") => Self {
                display_name: "HashiCorp Vault",
                emoji: "🔐",
                health_endpoint: None,
                health_port: Some(8200),
            },
            _ => Self {
                display_name: "Unknown",
                emoji: "❓",
                health_endpoint: None,
                health_port: None,
            },
        }
    }
}
