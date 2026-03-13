# PM-01: Project Plan
**Project Name:** Várðr — Asgard Monitoring Dashboard
**Document Version:** 1.0
**Date:** 2026-03-13
**Standard:** ISO/IEC 29110 — PM Process

---

## 1. Project Scope & Objectives

### เป้าหมาย
สร้าง monitoring dashboard สำหรับ Asgard AI Platform ที่แสดง service health, Docker logs, และ container metrics แบบ real-time

### Tech Stack
| Layer | Technology |
|:--|:--|
| Backend | Rust, Axum 0.8, Tokio |
| Frontend | Embedded HTML/CSS/JS (no npm) |
| Data Source | Docker CLI (ps, stats, logs) |
| Streaming | Server-Sent Events (SSE) |

### Features
| Feature | Description |
|:--|:--|
| 📊 Services | Container status, health badges, ports, uptime |
| 📜 Logs | Per-service logs, level filter, keyword search, SSE streaming |
| 📈 Metrics | CPU, Memory, Network I/O, PID count |

---

## 2. Project Schedule

| Sprint | Deliverable | Tests | Status |
|:--|:--|:--|:--|
| Sprint 1 | Foundation: Axum server, Docker CLI client, embedded UI, 3 tabs | 5 | ✅ Done (2026-03-13) |

---

## 3. Risk Management

| Risk | Impact | Mitigation |
|:--|:--|:--|
| Docker socket access in container | Medium | Mount /var/run/docker.sock read-only |
| Docker CLI not available in container | High | Install docker.io in Dockerfile |

---

*บันทึกโดย: AI Assistant (ตามมาตรฐาน ISO/IEC 29110 หมวด PM-01)*
