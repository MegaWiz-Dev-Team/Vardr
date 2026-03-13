use crate::models::ContainerMetrics;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

// ═══════════════════════════════════════
// 🚨 Alert Rules & Engine
// ═══════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertCondition {
    CpuAbove(f64),        // CPU % threshold
    MemoryAbove(f64),     // Memory % threshold
    ContainerDown,        // Container not running
    RestartLoop(u32),     // Restarted more than N times
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub service: String,       // "*" for all services, or specific name
    pub condition: AlertCondition,
    pub severity: AlertSeverity,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Alert {
    pub rule_id: String,
    pub rule_name: String,
    pub service: String,
    pub severity: AlertSeverity,
    pub message: String,
    pub timestamp: String,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertSummary {
    pub total_rules: usize,
    pub active_alerts: usize,
    pub critical: usize,
    pub warning: usize,
    pub info: usize,
}

pub struct AlertEngine {
    pub rules: Arc<RwLock<Vec<AlertRule>>>,
    pub active_alerts: Arc<RwLock<Vec<Alert>>>,
}

impl AlertEngine {
    pub fn new() -> Self {
        let default_rules = vec![
            AlertRule {
                id: "cpu-high".to_string(),
                name: "High CPU Usage".to_string(),
                service: "*".to_string(),
                condition: AlertCondition::CpuAbove(80.0),
                severity: AlertSeverity::Warning,
                enabled: true,
            },
            AlertRule {
                id: "cpu-critical".to_string(),
                name: "Critical CPU Usage".to_string(),
                service: "*".to_string(),
                condition: AlertCondition::CpuAbove(95.0),
                severity: AlertSeverity::Critical,
                enabled: true,
            },
            AlertRule {
                id: "mem-high".to_string(),
                name: "High Memory Usage".to_string(),
                service: "*".to_string(),
                condition: AlertCondition::MemoryAbove(85.0),
                severity: AlertSeverity::Warning,
                enabled: true,
            },
            AlertRule {
                id: "mem-critical".to_string(),
                name: "Critical Memory Usage".to_string(),
                service: "*".to_string(),
                condition: AlertCondition::MemoryAbove(95.0),
                severity: AlertSeverity::Critical,
                enabled: true,
            },
            AlertRule {
                id: "container-down".to_string(),
                name: "Container Down".to_string(),
                service: "*".to_string(),
                condition: AlertCondition::ContainerDown,
                severity: AlertSeverity::Critical,
                enabled: true,
            },
        ];

        Self {
            rules: Arc::new(RwLock::new(default_rules)),
            active_alerts: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Evaluate all rules against current metrics & service states
    pub async fn evaluate(
        &self,
        metrics: &[ContainerMetrics],
        service_states: &[(String, String)], // (service_name, state)
    ) {
        let rules = self.rules.read().await;
        let mut new_alerts = Vec::new();

        for rule in rules.iter() {
            if !rule.enabled {
                continue;
            }

            match &rule.condition {
                AlertCondition::CpuAbove(threshold) => {
                    for m in metrics {
                        if !matches_service(&rule.service, &m.service) {
                            continue;
                        }
                        if m.cpu_percent > *threshold {
                            new_alerts.push(Alert {
                                rule_id: rule.id.clone(),
                                rule_name: rule.name.clone(),
                                service: m.service.clone(),
                                severity: rule.severity.clone(),
                                message: format!(
                                    "{} CPU at {:.1}% (threshold: {:.0}%)",
                                    m.service, m.cpu_percent, threshold
                                ),
                                timestamp: chrono::Utc::now()
                                    .format("%Y-%m-%dT%H:%M:%SZ")
                                    .to_string(),
                                resolved: false,
                            });
                        }
                    }
                }
                AlertCondition::MemoryAbove(threshold) => {
                    for m in metrics {
                        if !matches_service(&rule.service, &m.service) {
                            continue;
                        }
                        if m.memory_percent > *threshold {
                            new_alerts.push(Alert {
                                rule_id: rule.id.clone(),
                                rule_name: rule.name.clone(),
                                service: m.service.clone(),
                                severity: rule.severity.clone(),
                                message: format!(
                                    "{} memory at {:.1}% ({:.0}MB / {:.0}MB)",
                                    m.service, m.memory_percent,
                                    m.memory_usage_mb, m.memory_limit_mb
                                ),
                                timestamp: chrono::Utc::now()
                                    .format("%Y-%m-%dT%H:%M:%SZ")
                                    .to_string(),
                                resolved: false,
                            });
                        }
                    }
                }
                AlertCondition::ContainerDown => {
                    for (name, state) in service_states {
                        if !matches_service(&rule.service, name) {
                            continue;
                        }
                        if state != "running" {
                            new_alerts.push(Alert {
                                rule_id: rule.id.clone(),
                                rule_name: rule.name.clone(),
                                service: name.clone(),
                                severity: rule.severity.clone(),
                                message: format!("{} is {} (not running)", name, state),
                                timestamp: chrono::Utc::now()
                                    .format("%Y-%m-%dT%H:%M:%SZ")
                                    .to_string(),
                                resolved: false,
                            });
                        }
                    }
                }
                AlertCondition::RestartLoop(max_restarts) => {
                    for (name, state) in service_states {
                        if !matches_service(&rule.service, name) {
                            continue;
                        }
                        if state == "restarting" {
                            new_alerts.push(Alert {
                                rule_id: rule.id.clone(),
                                rule_name: rule.name.clone(),
                                service: name.clone(),
                                severity: rule.severity.clone(),
                                message: format!(
                                    "{} in restart loop (threshold: {} restarts)",
                                    name, max_restarts
                                ),
                                timestamp: chrono::Utc::now()
                                    .format("%Y-%m-%dT%H:%M:%SZ")
                                    .to_string(),
                                resolved: false,
                            });
                        }
                    }
                }
            }
        }

        // Replace active alerts
        let mut active = self.active_alerts.write().await;
        *active = new_alerts;
    }

    /// Get alert summary
    pub async fn summary(&self) -> AlertSummary {
        let rules = self.rules.read().await;
        let alerts = self.active_alerts.read().await;

        let critical = alerts
            .iter()
            .filter(|a| matches!(a.severity, AlertSeverity::Critical))
            .count();
        let warning = alerts
            .iter()
            .filter(|a| matches!(a.severity, AlertSeverity::Warning))
            .count();
        let info = alerts
            .iter()
            .filter(|a| matches!(a.severity, AlertSeverity::Info))
            .count();

        AlertSummary {
            total_rules: rules.len(),
            active_alerts: alerts.len(),
            critical,
            warning,
            info,
        }
    }
}

fn matches_service(pattern: &str, service: &str) -> bool {
    pattern == "*" || pattern == service || service.contains(pattern)
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_service() {
        assert!(matches_service("*", "mimir_api"));
        assert!(matches_service("mimir_api", "mimir_api"));
        assert!(matches_service("mimir", "mimir_api"));
        assert!(!matches_service("bifrost", "mimir_api"));
    }

    #[test]
    fn test_alert_severity_serialize() {
        let alert = Alert {
            rule_id: "cpu-high".to_string(),
            rule_name: "High CPU".to_string(),
            service: "redis".to_string(),
            severity: AlertSeverity::Warning,
            message: "CPU at 85%".to_string(),
            timestamp: "2026-03-13T12:00:00Z".to_string(),
            resolved: false,
        };
        let json = serde_json::to_string(&alert).unwrap();
        assert!(json.contains("Warning"));
        assert!(json.contains("redis"));
    }

    #[tokio::test]
    async fn test_alert_engine_cpu() {
        let engine = AlertEngine::new();
        let metrics = vec![ContainerMetrics {
            service: "redis".to_string(),
            cpu_percent: 90.0,
            memory_percent: 50.0,
            memory_usage_mb: 100.0,
            memory_limit_mb: 200.0,
            ..Default::default()
        }];
        let states = vec![("redis".to_string(), "running".to_string())];
        engine.evaluate(&metrics, &states).await;
        let alerts = engine.active_alerts.read().await;
        // Should trigger cpu-high (80%) but not cpu-critical (95%)
        assert!(alerts.iter().any(|a| a.rule_id == "cpu-high"));
        assert!(!alerts.iter().any(|a| a.rule_id == "cpu-critical"));
    }

    #[tokio::test]
    async fn test_alert_engine_container_down() {
        let engine = AlertEngine::new();
        let metrics = vec![];
        let states = vec![("bifrost".to_string(), "exited".to_string())];
        engine.evaluate(&metrics, &states).await;
        let alerts = engine.active_alerts.read().await;
        assert!(alerts.iter().any(|a| a.rule_id == "container-down"));
        assert_eq!(alerts[0].service, "bifrost");
    }

    #[tokio::test]
    async fn test_alert_engine_all_healthy() {
        let engine = AlertEngine::new();
        let metrics = vec![ContainerMetrics {
            service: "redis".to_string(),
            cpu_percent: 5.0,
            memory_percent: 30.0,
            memory_usage_mb: 50.0,
            memory_limit_mb: 200.0,
            ..Default::default()
        }];
        let states = vec![("redis".to_string(), "running".to_string())];
        engine.evaluate(&metrics, &states).await;
        let alerts = engine.active_alerts.read().await;
        assert!(alerts.is_empty());
    }

    #[tokio::test]
    async fn test_alert_summary() {
        let engine = AlertEngine::new();
        let metrics = vec![ContainerMetrics {
            service: "neo4j".to_string(),
            cpu_percent: 96.0,
            memory_percent: 90.0,
            ..Default::default()
        }];
        let states = vec![
            ("neo4j".to_string(), "running".to_string()),
            ("fenrir".to_string(), "exited".to_string()),
        ];
        engine.evaluate(&metrics, &states).await;
        let summary = engine.summary().await;
        assert!(summary.active_alerts > 0);
        assert!(summary.critical > 0); // cpu-critical + container-down
    }

    #[tokio::test]
    async fn test_disabled_rule() {
        let engine = AlertEngine::new();
        // Disable all rules
        {
            let mut rules = engine.rules.write().await;
            for r in rules.iter_mut() {
                r.enabled = false;
            }
        }
        let metrics = vec![ContainerMetrics {
            service: "redis".to_string(),
            cpu_percent: 99.0,
            ..Default::default()
        }];
        let states = vec![];
        engine.evaluate(&metrics, &states).await;
        let alerts = engine.active_alerts.read().await;
        assert!(alerts.is_empty());
    }

    #[test]
    fn test_alert_condition_variants() {
        let cpu = AlertCondition::CpuAbove(80.0);
        let mem = AlertCondition::MemoryAbove(90.0);
        let down = AlertCondition::ContainerDown;
        let restart = AlertCondition::RestartLoop(3);
        assert_ne!(cpu, mem);
        assert_ne!(down, restart);
    }
}
