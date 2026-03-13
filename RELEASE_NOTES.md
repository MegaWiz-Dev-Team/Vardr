# Release Notes — Várðr

## v0.1.0 — Foundation (2026-03-13)

> Asgard เป็นของทุกคนแล้ว — Asgard belongs to everyone.

### ✨ New Features
- **Services Tab** — 10 container cards with health badges, ports, uptime, image info
- **Logs Tab** — per-service log viewer with level filter, keyword search, SSE real-time streaming
- **Metrics Tab** — CPU %, Memory usage/limit, Network RX/TX, PID count per container
- **Dark Theme** — premium glassmorphism UI with Inter + JetBrains Mono fonts
- **Embedded UI** — HTML/CSS/JS compiled into binary (no npm required)
- **Docker CLI Client** — uses `docker ps`, `docker stats`, `docker logs` for data collection
- **SSE Streaming** — Server-Sent Events for real-time log tail

### 🔒 Security
- Docker socket mounted read-only
- No external network dependencies
- Single binary deployment

### 📊 Stats
- **5 tests**, all passing
- 10 source files
- Port: :9090
- ISO 29110 documentation (PM×2)

---

*Asgard เป็นของทุกคนแล้ว — Asgard belongs to everyone.*
