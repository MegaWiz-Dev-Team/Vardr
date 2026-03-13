# 🛡️ Várðr — Asgard Monitoring Dashboard

Real-time monitoring dashboard for all Asgard services. Built with Rust (Axum).

## Features

- 📊 **Service Health** — Live status of all Docker Compose services
- 📜 **Log Viewer** — Per-service logs with level filter, keyword search, and real-time SSE streaming
- 📈 **Metrics** — CPU, Memory, Network I/O, PID count per container
- 🌙 **Dark Theme** — Premium glassmorphism UI

## Quick Start

```bash
cargo run
# → http://localhost:9090
```

## Docker

```bash
docker build -t vardr .
docker run -v /var/run/docker.sock:/var/run/docker.sock:ro -p 9090:9090 vardr
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
