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

- **Sprint 31: Mimir Hybrid Search & MCP Server Foundation** [Planned]
  - True Vector Integration, Parallel Tree Search, Neo4j Graph, Ensemble Retrieval, and Rust MCP Server.
- **Sprint 32: Asgard/Bifrost MCP Adapter & Dynamic Tenants** [Planned]
  - Auto-discover tools from MCP servers, Dynamic Context Isolation (X-Tenant-ID), Agent-to-Agent via JSON-RPC.
- **Sprint 33: Ecosystem Gateway Sidecars** [Planned]
  - Yggdrasil & Eir Universal Go Sidecars to expose auth and medical tools to Asgard.
- **Sprint 34: Platform Automation (Testing, Browsing & Security)** [Planned]
  - Deploy MCP across Fenrir, Forseti, Ratatoskr, Huginn, Muninn, and Heimdall.
