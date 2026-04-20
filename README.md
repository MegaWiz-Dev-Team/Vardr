# 🛡️ Várðr — Asgard Monitoring Dashboard

Real-time monitoring dashboard for all Asgard services. Built with Rust (Axum).

### 🏥 Role in Multi-Agent Ecosystem

> **Observability & Clinical Tracing (ยามรักษาการณ์)** — Várðr trace ทุก Agent call chain ด้วย **OpenTelemetry** เพื่อให้แพทย์สามารถดูย้อนหลังว่า "AI คิดยังไง" — ทุก Trace เชื่อมกลับไปหา Audit Trail ได้
>
> **Integrations:** Prometheus (Metrics) • Grafana (Dashboard) • Structured JSON Logs
>
> 📖 [Full Architecture →](https://github.com/MegaWiz-Dev-Team/Asgard/blob/main/docs/roadmap/MultiAgent_Architecture_Plan.md) | [Sprint Plan →](https://github.com/MegaWiz-Dev-Team/Asgard/blob/main/docs/roadmap/MultiAgent_Sprint_Plan.md)

## Features

- 📊 **Service Health** — Live status of all Docker Compose services
- 📜 **Log Viewer** — Per-service logs with level filter, keyword search, and real-time SSE streaming
- 📈 **Metrics** — CPU, Memory, Network I/O, PID count per container
- 🌙 **Dark Theme** — Premium glassmorphism UI

## Quick Start

### 1. Deploy the K3s Dashboard
```bash
docker build -t vardr .
# Várðr runs natively in Kubernetes but accesses host stats via the Agent
docker run -v /var/run/docker.sock:/var/run/docker.sock:ro -p 9090:9090 vardr
```

### 2. Install the Native macOS Host Agent
Because Várðr is containerized and `Heimdall Gateway` runs as a bare-metal macOS `launchd` process (to access Apple Silicon GPU unified memory), Várðr cannot natively read its CPU utilization or files. 
You must install the **Várðr Native Agent**:

```bash
# Compiles and deploys the agent to macOS launchd (Port 9091)
sh ./setup_mac_vardr_agent.sh
```

## Tech Stack

- **Backend:** Rust, Axum 0.8, Tokio
- **Frontend:** Embedded HTML/CSS/JS (no npm)
- **Data:** Docker CLI (`docker ps`, `docker stats`, `docker logs`)

## Part of Asgard

| Component | Description |
|:--|:--|
| 🧠 Mimir | RAG Pipeline + Agent Builder |
| ⚡ Bifrost | Agent Runtime Engine |
| 🐺 Fenrir | Computer-Use Agent |
| 🌳 Yggdrasil | Auth Service (Zitadel) |
| 🛡️ **Várðr** | **Monitoring Dashboard** |
| 🏥 Eir | API Gateway |
| 🛡️ Heimdall | LLM Gateway |

## License

AGPL-3.0
