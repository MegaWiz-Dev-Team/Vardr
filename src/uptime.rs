// ═══════════════════════════════════════
// ⏱️ Uptime History Tracker — Sprint 3
// ═══════════════════════════════════════
//
// Tracks per-container uptime, state transitions, and auto-restart counts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Per-container uptime record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UptimeRecord {
    pub container: String,
    pub uptime_seconds: u64,
    pub last_started: Option<DateTime<Utc>>,
    pub last_stopped: Option<DateTime<Utc>>,
    pub auto_restart_count: u32,
    pub state_history: Vec<StateTransition>,
}

/// A single state transition event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub timestamp: DateTime<Utc>,
    pub from: ContainerState,
    pub to: ContainerState,
}

/// Container states for tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerState {
    Running,
    Stopped,
    Restarting,
    Unknown,
}

impl std::fmt::Display for ContainerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => write!(f, "running"),
            Self::Stopped => write!(f, "stopped"),
            Self::Restarting => write!(f, "restarting"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Thread-safe uptime tracker.
pub struct UptimeTracker {
    pub records: RwLock<HashMap<String, UptimeRecord>>,
    previous_states: RwLock<HashMap<String, ContainerState>>,
}

impl UptimeTracker {
    pub fn new() -> Self {
        Self {
            records: RwLock::new(HashMap::new()),
            previous_states: RwLock::new(HashMap::new()),
        }
    }

    /// Update uptime for a container based on its current status string.
    pub async fn update(&self, container: &str, status: &str) {
        let current_state = Self::parse_state(status);
        let now = Utc::now();

        let mut records = self.records.write().await;
        let mut prev_states = self.previous_states.write().await;

        let record = records.entry(container.to_string()).or_insert_with(|| UptimeRecord {
            container: container.to_string(),
            uptime_seconds: 0,
            last_started: None,
            last_stopped: None,
            auto_restart_count: 0,
            state_history: Vec::new(),
        });

        let prev = prev_states.get(container).copied().unwrap_or(ContainerState::Unknown);

        // Detect state transition
        if prev != current_state && prev != ContainerState::Unknown {
            record.state_history.push(StateTransition {
                timestamp: now,
                from: prev,
                to: current_state,
            });

            // Keep only last 50 transitions
            if record.state_history.len() > 50 {
                record.state_history.drain(..record.state_history.len() - 50);
            }

            // Track start/stop times
            match current_state {
                ContainerState::Running => {
                    record.last_started = Some(now);
                    tracing::info!("⬆️ {} is now running", container);
                }
                ContainerState::Stopped => {
                    record.last_stopped = Some(now);
                    tracing::warn!("⬇️ {} stopped", container);
                }
                _ => {}
            }
        }

        // Accumulate uptime if running
        if current_state == ContainerState::Running {
            record.uptime_seconds += 15; // poll interval
        }

        prev_states.insert(container.to_string(), current_state);
    }

    /// Record an auto-restart event.
    pub async fn record_auto_restart(&self, container: &str) {
        let mut records = self.records.write().await;
        if let Some(record) = records.get_mut(container) {
            record.auto_restart_count += 1;
        }
    }

    /// Get all uptime records.
    pub async fn get_all_uptime(&self) -> HashMap<String, UptimeRecord> {
        self.records.read().await.clone()
    }

    /// Get uptime for a specific container.
    pub async fn get_uptime(&self, container: &str) -> Option<UptimeRecord> {
        self.records.read().await.get(container).cloned()
    }

    fn parse_state(status: &str) -> ContainerState {
        let lower = status.to_lowercase();
        if lower.contains("up") {
            ContainerState::Running
        } else if lower.contains("exited") || lower.contains("stopped") || lower.contains("dead") {
            ContainerState::Stopped
        } else if lower.contains("restarting") {
            ContainerState::Restarting
        } else {
            ContainerState::Unknown
        }
    }
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_uptime_new_container() {
        let tracker = UptimeTracker::new();
        tracker.update("test_container", "Up 2 hours").await;
        let records = tracker.records.read().await;
        assert!(records.contains_key("test_container"));
    }

    #[tokio::test]
    async fn test_uptime_accumulates() {
        let tracker = UptimeTracker::new();
        // First update initializes (unknown → running), no accumulation yet for first
        tracker.update("test", "Up 1 hour").await;
        let r1 = tracker.records.read().await.get("test").unwrap().uptime_seconds;

        drop(tracker.records.read().await);
        // Second update accumulates
        tracker.update("test", "Up 1 hour").await;
        let r2 = tracker.records.read().await.get("test").unwrap().uptime_seconds;
        assert!(r2 > r1, "Uptime should accumulate: {} > {}", r2, r1);
    }

    #[tokio::test]
    async fn test_state_transition_detected() {
        let tracker = UptimeTracker::new();
        tracker.update("svc", "Up 1 hour").await;
        tracker.update("svc", "Exited (0) 5 seconds ago").await;

        let records = tracker.records.read().await;
        let svc = records.get("svc").unwrap();
        assert!(!svc.state_history.is_empty(), "Should have state transitions");
        assert!(svc.last_stopped.is_some(), "Should have last_stopped");
    }

    #[tokio::test]
    async fn test_auto_restart_counter() {
        let tracker = UptimeTracker::new();
        tracker.update("svc", "Up").await;
        tracker.record_auto_restart("svc").await;
        tracker.record_auto_restart("svc").await;

        let records = tracker.records.read().await;
        assert_eq!(records.get("svc").unwrap().auto_restart_count, 2);
    }

    #[tokio::test]
    async fn test_parse_state_variants() {
        assert_eq!(UptimeTracker::parse_state("Up 2 hours (healthy)"), ContainerState::Running);
        assert_eq!(UptimeTracker::parse_state("Exited (0) 5 seconds ago"), ContainerState::Stopped);
        assert_eq!(UptimeTracker::parse_state("Restarting (1) 3 seconds ago"), ContainerState::Restarting);
        assert_eq!(UptimeTracker::parse_state("Created"), ContainerState::Unknown);
    }

    #[tokio::test]
    async fn test_history_capped_at_50() {
        let tracker = UptimeTracker::new();
        tracker.update("svc", "Up").await;
        for i in 0..60 {
            if i % 2 == 0 {
                tracker.update("svc", "Exited (0)").await;
            } else {
                tracker.update("svc", "Up").await;
            }
        }
        let records = tracker.records.read().await;
        assert!(records.get("svc").unwrap().state_history.len() <= 50);
    }
}
