// ═══════════════════════════════════════
// 🔄 Auto-Restart Watchdog — Sprint 3
// ═══════════════════════════════════════
//
// Watches for crashed containers and automatically restarts them.

use crate::docker::DockerClient;
use crate::uptime::UptimeTracker;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for auto-restart behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoRestartConfig {
    pub enabled: bool,
    pub max_restarts: u32,
    pub cooldown_seconds: u64,
    pub watched_containers: HashSet<String>,
}

impl Default for AutoRestartConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_restarts: 3,
            cooldown_seconds: 60,
            watched_containers: HashSet::new(), // empty = watch all asgard_*
        }
    }
}

/// A logged auto-restart event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestartEvent {
    pub container: String,
    pub timestamp: DateTime<Utc>,
    pub reason: String,
    pub success: bool,
}

/// Auto-restart watchdog service.
pub struct AutoRestartWatchdog {
    pub config: RwLock<AutoRestartConfig>,
    pub events: RwLock<Vec<RestartEvent>>,
    restart_counts: RwLock<HashMap<String, (u32, DateTime<Utc>)>>, // (count, last_restart)
}

impl AutoRestartWatchdog {
    pub fn new() -> Self {
        Self {
            config: RwLock::new(AutoRestartConfig::default()),
            events: RwLock::new(Vec::new()),
            restart_counts: RwLock::new(HashMap::new()),
        }
    }

    /// Check all containers and restart any that have crashed.
    pub async fn check_and_restart(
        &self,
        docker: &DockerClient,
        uptime: &Arc<UptimeTracker>,
    ) -> Vec<RestartEvent> {
        let config = self.config.read().await;
        if !config.enabled {
            return Vec::new();
        }

        let services = docker.list_services().await;
        let mut new_events = Vec::new();

        for svc in &services {
            // Only watch asgard_* containers
            if !svc.name.starts_with("asgard_") {
                continue;
            }

            // Skip if not in watched list (unless list is empty = watch all)
            if !config.watched_containers.is_empty()
                && !config.watched_containers.contains(&svc.name)
            {
                continue;
            }

            let status_lower = svc.status.to_lowercase();
            let needs_restart = status_lower.contains("exited")
                || status_lower.contains("dead")
                || (status_lower.contains("unhealthy")
                    && !status_lower.contains("starting"));

            if needs_restart {
                let event = self
                    .try_restart(&svc.name, &svc.status, docker, uptime, &config)
                    .await;
                if let Some(evt) = event {
                    new_events.push(evt);
                }
            }
        }

        // Store events
        if !new_events.is_empty() {
            let mut events = self.events.write().await;
            events.extend(new_events.clone());
            // Keep last 100 events
            let excess = events.len().saturating_sub(100);
            if excess > 0 {
                events.drain(..excess);
            }
        }

        new_events
    }

    async fn try_restart(
        &self,
        container: &str,
        status: &str,
        docker: &DockerClient,
        uptime: &Arc<UptimeTracker>,
        config: &AutoRestartConfig,
    ) -> Option<RestartEvent> {
        let now = Utc::now();
        let mut counts = self.restart_counts.write().await;

        let (count, last_time) = counts
            .entry(container.to_string())
            .or_insert((0, now - chrono::Duration::hours(1)));

        // Check cooldown
        let elapsed = (now - *last_time).num_seconds() as u64;
        if elapsed < config.cooldown_seconds {
            tracing::debug!(
                "⏳ {} cooldown: {}s remaining",
                container,
                config.cooldown_seconds - elapsed
            );
            return None;
        }

        // Check max restarts
        if *count >= config.max_restarts {
            tracing::warn!(
                "🚫 {} exceeded max restarts ({}), skipping",
                container,
                config.max_restarts
            );
            return Some(RestartEvent {
                container: container.to_string(),
                timestamp: now,
                reason: format!("Max restarts ({}) exceeded", config.max_restarts),
                success: false,
            });
        }

        // Attempt restart
        tracing::info!("🔄 Auto-restarting crashed container: {} (was: {})", container, status);
        let result = docker.restart_container(container).await;

        let success = result.is_ok();
        let reason = if success {
            format!("Auto-restarted (was: {})", status)
        } else {
            format!("Restart failed: {}", result.unwrap_err())
        };

        *count += 1;
        *last_time = now;

        // Record in uptime tracker
        if success {
            uptime.record_auto_restart(container).await;
        }

        Some(RestartEvent {
            container: container.to_string(),
            timestamp: now,
            reason,
            success,
        })
    }

    /// Get status summary.
    pub async fn status(&self) -> serde_json::Value {
        let config = self.config.read().await;
        let events = self.events.read().await;
        let recent = events.iter().rev().take(10).cloned().collect::<Vec<_>>();

        serde_json::json!({
            "enabled": config.enabled,
            "max_restarts": config.max_restarts,
            "cooldown_seconds": config.cooldown_seconds,
            "total_events": events.len(),
            "recent_events": recent,
        })
    }
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AutoRestartConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_restarts, 3);
        assert_eq!(config.cooldown_seconds, 60);
        assert!(config.watched_containers.is_empty());
    }

    #[test]
    fn test_restart_event_serialization() {
        let event = RestartEvent {
            container: "asgard_mimir".to_string(),
            timestamp: Utc::now(),
            reason: "Exited (1)".to_string(),
            success: true,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("asgard_mimir"));
        assert!(json.contains("\"success\":true"));
    }

    #[tokio::test]
    async fn test_watchdog_disabled() {
        let watchdog = AutoRestartWatchdog::new();
        {
            let mut config = watchdog.config.write().await;
            config.enabled = false;
        }
        let docker = DockerClient::new();
        let uptime = Arc::new(UptimeTracker::new());
        let events = watchdog.check_and_restart(&docker, &uptime).await;
        assert!(events.is_empty(), "Should not restart when disabled");
    }

    #[tokio::test]
    async fn test_watchdog_status() {
        let watchdog = AutoRestartWatchdog::new();
        let status = watchdog.status().await;
        assert_eq!(status["enabled"], true);
        assert_eq!(status["max_restarts"], 3);
        assert_eq!(status["total_events"], 0);
    }

    #[tokio::test]
    async fn test_events_capped() {
        let watchdog = AutoRestartWatchdog::new();
        let mut events = watchdog.events.write().await;
        for i in 0..120 {
            events.push(RestartEvent {
                container: format!("test_{}", i),
                timestamp: Utc::now(),
                reason: "test".to_string(),
                success: true,
            });
        }
        // Cap
        let excess = events.len().saturating_sub(100);
        if excess > 0 {
            events.drain(..excess);
        }
        assert!(events.len() <= 100);
    }
}
