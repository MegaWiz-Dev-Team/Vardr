# Release Notes — Várðr

## v0.2.0 — Container Controls & Alerts (2026-03-13)

### ✨ New Features
- **Container Controls** — restart, stop, start any container from the UI
- **Docker Compose Controls** — Up, Restart, Down buttons for full stack management
- **Alert Engine** — 5 built-in rules (CPU/Memory thresholds, Container Down)
- **Alerts Tab** — real-time summary cards, active alerts list, rule management
- **Background Evaluator** — checks every 15 seconds
- **Alert API** — REST endpoints for alerts, rules, and summary
- **Toast Notifications** — visual feedback for all container actions

### 📊 Stats
- **16 tests**, all passing (8 alerts + 8 docker)
- **17 API endpoints** (12 new)
- Sprint 2 complete

---

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
