# Sprint 2 Report — Várðr

| Field | Value |
|:--|:--|
| Sprint | 2 — Container Controls & Alert Engine |
| Period | 2026-03-13 |
| Version | v0.2.0 |
| Status | ✅ Complete |

## Deliverables

### Container Controls
- `POST /api/containers/{name}/restart` — Restart container
- `POST /api/containers/{name}/stop` — Stop container
- `POST /api/containers/{name}/start` — Start container

### Docker Compose Controls
- `POST /api/compose/up` — `docker compose up -d`
- `POST /api/compose/down` — `docker compose down`
- `POST /api/compose/restart` — `docker compose restart`

### Alert Engine
- 5 default rules: CPU > 80%/95%, Memory > 85%/95%, Container Down
- Background evaluator (15s interval)
- `GET /api/alerts` — Active alerts
- `GET /api/alerts/summary` — Alert summary
- `GET /api/alerts/rules` — Rule list
- `POST /api/alerts/rules` — Add custom rule

### Frontend
- Restart / Stop / Start buttons on each service card
- Docker Compose Up / Restart / Down bar
- Alerts tab with summary cards + active alerts + rules list
- Toast notifications for action feedback

## Metrics

| Metric | Value |
|:--|:--|
| Tests | 16 (8 alerts + 8 docker) |
| New files | 1 (`src/alerts.rs`) |
| Modified files | 3 (`main.rs`, `docker.rs`, frontend ×3) |
| API endpoints | 12 new (Sprint 1: 5 → Sprint 2: 17 total) |
| LOC added | ~700 |

## Files

| File | Lines |
|:--|:--|
| `src/alerts.rs` | 290 |
| `src/main.rs` | 280 |
| `src/docker.rs` | 415 |
| `src/models.rs` | 236 |
| `static/index.html` | 97 |
| `static/style.css` | 248 |
| `static/app.js` | 295 |
