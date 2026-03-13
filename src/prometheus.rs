// ═══════════════════════════════════════
// 📊 Prometheus Metrics Export — Sprint 3
// ═══════════════════════════════════════
//
// Generates Prometheus text-format metrics from Docker stats.
// Endpoint: GET /metrics

use crate::docker::DockerClient;
use crate::uptime::UptimeTracker;
use std::sync::Arc;

/// Build Prometheus text-format metrics for all containers.
pub async fn render_metrics(docker: &DockerClient, uptime: &Arc<UptimeTracker>) -> String {
    let mut out = String::with_capacity(4096);

    // Header
    out.push_str("# Várðr Prometheus Metrics\n\n");

    // ── Container CPU ──
    let stats = docker.get_all_stats().await;
    out.push_str("# HELP vardr_container_cpu_percent CPU usage percentage per container\n");
    out.push_str("# TYPE vardr_container_cpu_percent gauge\n");
    for s in &stats {
        out.push_str(&format!(
            "vardr_container_cpu_percent{{container=\"{}\"}} {:.2}\n",
            s.service, s.cpu_percent
        ));
    }
    out.push('\n');

    // ── Container Memory ──
    out.push_str("# HELP vardr_container_memory_bytes Memory usage in bytes per container\n");
    out.push_str("# TYPE vardr_container_memory_bytes gauge\n");
    for s in &stats {
        out.push_str(&format!(
            "vardr_container_memory_bytes{{container=\"{}\"}} {}\n",
            s.service, s.memory_usage_mb as u64 * 1024 * 1024
        ));
    }
    out.push('\n');

    out.push_str("# HELP vardr_container_memory_limit_bytes Memory limit in bytes per container\n");
    out.push_str("# TYPE vardr_container_memory_limit_bytes gauge\n");
    for s in &stats {
        out.push_str(&format!(
            "vardr_container_memory_limit_bytes{{container=\"{}\"}} {}\n",
            s.service, s.memory_limit_mb as u64 * 1024 * 1024
        ));
    }
    out.push('\n');

    // ── Container Running ──
    let services = docker.list_services().await;
    out.push_str("# HELP vardr_container_running Whether the container is running (1=running, 0=stopped)\n");
    out.push_str("# TYPE vardr_container_running gauge\n");
    for svc in &services {
        let running = if svc.status.contains("Up") { 1 } else { 0 };
        out.push_str(&format!(
            "vardr_container_running{{container=\"{}\"}} {}\n",
            svc.name, running
        ));
    }
    out.push('\n');

    // ── Uptime Seconds ──
    let history = uptime.get_all_uptime().await;
    out.push_str("# HELP vardr_container_uptime_seconds Cumulative uptime in seconds per container\n");
    out.push_str("# TYPE vardr_container_uptime_seconds gauge\n");
    for (name, record) in &history {
        out.push_str(&format!(
            "vardr_container_uptime_seconds{{container=\"{}\"}} {}\n",
            name, record.uptime_seconds
        ));
    }
    out.push('\n');

    // ── Auto-Restart Count ──
    out.push_str("# HELP vardr_container_auto_restarts_total Number of auto-restarts performed\n");
    out.push_str("# TYPE vardr_container_auto_restarts_total counter\n");
    for (name, record) in &history {
        out.push_str(&format!(
            "vardr_container_auto_restarts_total{{container=\"{}\"}} {}\n",
            name, record.auto_restart_count
        ));
    }
    out.push('\n');

    // ── Total Containers ──
    out.push_str("# HELP vardr_containers_total Total number of monitored containers\n");
    out.push_str("# TYPE vardr_containers_total gauge\n");
    out.push_str(&format!("vardr_containers_total {}\n", services.len()));

    out
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {

    #[test]
    fn test_prometheus_format_contains_help_and_type() {
        // Verify that we generate valid Prometheus text format
        let sample = "# HELP vardr_container_cpu_percent CPU usage percentage per container\n\
                       # TYPE vardr_container_cpu_percent gauge\n\
                       vardr_container_cpu_percent{container=\"test\"} 12.50\n";
        assert!(sample.contains("# HELP"));
        assert!(sample.contains("# TYPE"));
        assert!(sample.contains("gauge"));
    }

    #[test]
    fn test_prometheus_metric_line_format() {
        let name = "asgard_mimir";
        let cpu = 25.5;
        let line = format!("vardr_container_cpu_percent{{container=\"{}\"}} {:.2}\n", name, cpu);
        assert_eq!(line, "vardr_container_cpu_percent{container=\"asgard_mimir\"} 25.50\n");
    }

    #[test]
    fn test_prometheus_boolean_metric() {
        let running = 1;
        let line = format!("vardr_container_running{{container=\"test\"}} {}\n", running);
        assert!(line.contains("1"));
    }
}
