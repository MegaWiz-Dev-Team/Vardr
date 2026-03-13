// ═══════════════════════════════════════
// 🛡️ Várðr — Dashboard App v2
// ═══════════════════════════════════════

const API = '';
let currentTab = 'services';
let logStream = null;

// ── Init ──
document.addEventListener('DOMContentLoaded', () => {
    setupTabs();
    loadServices();
    loadAlertBadge();
    setInterval(() => {
        if (currentTab === 'services') loadServices();
        if (currentTab === 'metrics') loadMetrics();
        loadAlertBadge();
    }, 10000);
});

// ── Tabs ──
function setupTabs() {
    document.querySelectorAll('.tab').forEach(tab => {
        tab.addEventListener('click', () => {
            document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
            document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
            tab.classList.add('active');
            const target = tab.dataset.tab;
            document.getElementById(target).classList.add('active');
            currentTab = target;
            if (target === 'logs') loadServiceSelector();
            if (target === 'metrics') loadMetrics();
            if (target === 'alerts') loadAlerts();
        });
    });
    document.getElementById('log-fetch').addEventListener('click', fetchLogs);
    document.getElementById('log-stream-toggle').addEventListener('click', toggleStream);
}

// ═══════════════════════════════════════
// Services
// ═══════════════════════════════════════

async function loadServices() {
    try {
        const resp = await fetch(`${API}/api/services`);
        const services = await resp.json();
        renderServices(services);
        updateTimestamp();
    } catch (e) { console.error('Failed to load services:', e); }
}

function renderServices(services) {
    const grid = document.getElementById('services-grid');
    grid.innerHTML = services.map(s => {
        const stateClass = getStateClass(s.state, s.status);
        const statusBadge = getStatusBadge(s.state, s.status);
        const portsHtml = s.ports.map(p =>
            p.external ? `<span class="card-detail-value">:${p.external} → :${p.internal}</span>` : ''
        ).filter(Boolean).join(', ') || '<span class="card-detail-value">internal</span>';

        return `
            <div class="service-card ${stateClass}">
                <div class="card-header">
                    <div class="card-title">
                        <span class="card-emoji">${s.emoji}</span>
                        <span class="card-name">${s.display_name}</span>
                    </div>
                    ${statusBadge}
                </div>
                <div class="card-details">
                    <div class="card-detail">
                        <span class="card-detail-label">Status</span>
                        <span class="card-detail-value">${s.status}</span>
                    </div>
                    <div class="card-detail">
                        <span class="card-detail-label">Ports</span>
                        ${portsHtml}
                    </div>
                    <div class="card-detail">
                        <span class="card-detail-label">Image</span>
                        <span class="card-detail-value">${truncate(s.image, 35)}</span>
                    </div>
                </div>
                <div class="card-actions">
                    <button class="btn btn-secondary" onclick="viewServiceLogs('${s.name}')">📜 Logs</button>
                    <button class="btn btn-yellow" onclick="containerAction('${s.name}', 'restart')">🔄</button>
                    ${s.state === 'running'
                        ? `<button class="btn btn-red" onclick="containerAction('${s.name}', 'stop')">⏹</button>`
                        : `<button class="btn btn-green" onclick="containerAction('${s.name}', 'start')">▶</button>`
                    }
                </div>
            </div>
        `;
    }).join('');
}

function getStateClass(state, status) {
    if (status.toLowerCase().includes('unhealthy') || state === 'exited') return 'stopped';
    if (state === 'restarting') return 'restarting';
    return '';
}

function getStatusBadge(state, status) {
    if (status.toLowerCase().includes('healthy')) return '<span class="status-badge status-healthy">Healthy</span>';
    if (status.toLowerCase().includes('unhealthy')) return '<span class="status-badge status-stopped">Unhealthy</span>';
    if (state === 'running') return '<span class="status-badge status-running">Running</span>';
    if (state === 'restarting') return '<span class="status-badge status-restarting">Restarting</span>';
    if (state === 'exited') return '<span class="status-badge status-stopped">Stopped</span>';
    return `<span class="status-badge status-running">${state}</span>`;
}

// ═══════════════════════════════════════
// Container Controls (Sprint 2)
// ═══════════════════════════════════════

async function containerAction(name, action) {
    try {
        showToast(`${action === 'restart' ? '🔄' : action === 'stop' ? '⏹' : '▶'} ${action}ing ${name}...`, 'info');
        const resp = await fetch(`${API}/api/containers/${name}/${action}`, { method: 'POST' });
        const data = await resp.json();
        if (data.status === 'ok') {
            showToast(`✅ ${name} ${action}ed successfully`, 'success');
            setTimeout(loadServices, 1500);
        } else {
            showToast(`❌ Failed: ${data.message}`, 'error');
        }
    } catch (e) { showToast(`❌ Error: ${e.message}`, 'error'); }
}

async function composeAction(action) {
    try {
        showToast(`🐳 Docker Compose ${action}...`, 'info');
        const resp = await fetch(`${API}/api/compose/${action}`, { method: 'POST' });
        const data = await resp.json();
        if (data.status === 'ok') {
            showToast(`✅ Compose ${action} completed`, 'success');
            setTimeout(loadServices, 3000);
        } else {
            showToast(`❌ Compose ${action} failed: ${data.message}`, 'error');
        }
    } catch (e) { showToast(`❌ Error: ${e.message}`, 'error'); }
}

// ═══════════════════════════════════════
// Logs
// ═══════════════════════════════════════

async function loadServiceSelector() {
    try {
        const resp = await fetch(`${API}/api/services`);
        const services = await resp.json();
        const select = document.getElementById('log-service');
        const current = select.value;
        select.innerHTML = '<option value="">Select service...</option>' +
            services.map(s => `<option value="${s.name}" ${s.name === current ? 'selected' : ''}>${s.emoji} ${s.display_name}</option>`).join('');
    } catch (e) { console.error('Failed to load service list:', e); }
}

async function fetchLogs() {
    const service = document.getElementById('log-service').value;
    if (!service) return;
    const level = document.getElementById('log-level').value;
    const search = document.getElementById('log-search').value;
    try {
        const params = new URLSearchParams({ tail: '200' });
        if (level !== 'ALL') params.set('level', level);
        if (search) params.set('search', search);
        const resp = await fetch(`${API}/api/services/${service}/logs?${params}`);
        const logs = await resp.json();
        renderLogs(logs);
    } catch (e) { console.error('Failed to fetch logs:', e); }
}

function renderLogs(logs) {
    const viewer = document.getElementById('log-viewer');
    if (logs.length === 0) { viewer.innerHTML = '<div class="log-placeholder">No logs found</div>'; return; }
    viewer.innerHTML = logs.map(l => `
        <div class="log-line">
            <span class="log-ts">${formatTimestamp(l.timestamp)}</span>
            <span class="log-level log-level-${l.level}">${l.level}</span>
            <span class="log-msg">${escapeHtml(l.message)}</span>
        </div>
    `).join('');
    if (document.getElementById('log-autoscroll').checked) viewer.scrollTop = viewer.scrollHeight;
}

function viewServiceLogs(serviceName) {
    document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
    document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
    document.getElementById('tab-logs').classList.add('active');
    document.getElementById('logs').classList.add('active');
    currentTab = 'logs';
    loadServiceSelector().then(() => {
        document.getElementById('log-service').value = serviceName;
        fetchLogs();
    });
}

function toggleStream() {
    const btn = document.getElementById('log-stream-toggle');
    if (logStream) { logStream.close(); logStream = null; btn.textContent = '▶ Stream'; btn.classList.remove('active'); return; }
    const service = document.getElementById('log-service').value;
    if (!service) return;
    btn.textContent = '⏹ Stop'; btn.classList.add('active');
    logStream = new EventSource(`${API}/api/logs/stream/${service}`);
    logStream.onmessage = (event) => {
        try { appendLogEntry(JSON.parse(event.data)); } catch (e) { console.error('SSE parse:', e); }
    };
    logStream.onerror = () => { logStream.close(); logStream = null; btn.textContent = '▶ Stream'; btn.classList.remove('active'); };
}

function appendLogEntry(entry) {
    const viewer = document.getElementById('log-viewer');
    const placeholder = viewer.querySelector('.log-placeholder');
    if (placeholder) placeholder.remove();
    const div = document.createElement('div');
    div.className = 'log-line';
    div.innerHTML = `<span class="log-ts">${formatTimestamp(entry.timestamp)}</span><span class="log-level log-level-${entry.level}">${entry.level}</span><span class="log-msg">${escapeHtml(entry.message)}</span>`;
    viewer.appendChild(div);
    while (viewer.children.length > 1000) viewer.removeChild(viewer.firstChild);
    if (document.getElementById('log-autoscroll').checked) viewer.scrollTop = viewer.scrollHeight;
}

// ═══════════════════════════════════════
// Metrics
// ═══════════════════════════════════════

async function loadMetrics() {
    try {
        const resp = await fetch(`${API}/api/metrics`);
        const metrics = await resp.json();
        renderMetrics(metrics);
    } catch (e) { console.error('Failed to load metrics:', e); }
}

function renderMetrics(metrics) {
    const grid = document.getElementById('metrics-grid');
    if (metrics.length === 0) { grid.innerHTML = '<div class="loading">No metrics available</div>'; return; }
    grid.innerHTML = metrics.map(m => `
        <div class="metric-card">
            <div class="metric-header">
                <span class="metric-name">${m.service}</span>
                <span class="metric-pids">${m.pids} PIDs</span>
            </div>
            <div class="metric-bars">
                <div class="metric-row">
                    <div class="metric-label"><span>CPU</span><span>${m.cpu_percent.toFixed(2)}%</span></div>
                    <div class="metric-bar-bg"><div class="metric-bar metric-bar-cpu" style="width: ${Math.min(m.cpu_percent, 100)}%"></div></div>
                </div>
                <div class="metric-row">
                    <div class="metric-label"><span>Memory</span><span>${m.memory_usage_mb.toFixed(1)} / ${m.memory_limit_mb.toFixed(0)} MB (${m.memory_percent.toFixed(1)}%)</span></div>
                    <div class="metric-bar-bg"><div class="metric-bar metric-bar-mem" style="width: ${Math.min(m.memory_percent, 100)}%"></div></div>
                </div>
                <div class="metric-network">
                    <span class="metric-net-item">↓ RX: <span>${m.network_rx_mb.toFixed(2)} MB</span></span>
                    <span class="metric-net-item">↑ TX: <span>${m.network_tx_mb.toFixed(2)} MB</span></span>
                </div>
            </div>
        </div>
    `).join('');
}

// ═══════════════════════════════════════
// Alerts (Sprint 2)
// ═══════════════════════════════════════

async function loadAlertBadge() {
    try {
        const resp = await fetch(`${API}/api/alerts/summary`);
        const summary = await resp.json();
        const badge = document.getElementById('alert-badge');
        if (summary.active_alerts > 0) {
            badge.textContent = summary.active_alerts;
            badge.classList.add('visible');
        } else {
            badge.classList.remove('visible');
        }
    } catch (e) { /* silent */ }
}

async function loadAlerts() {
    try {
        const [summaryResp, alertsResp, rulesResp] = await Promise.all([
            fetch(`${API}/api/alerts/summary`),
            fetch(`${API}/api/alerts`),
            fetch(`${API}/api/alerts/rules`),
        ]);
        const summary = await summaryResp.json();
        const alerts = await alertsResp.json();
        const rules = await rulesResp.json();
        renderAlertSummary(summary);
        renderAlertsList(alerts);
        renderRulesList(rules);
    } catch (e) { console.error('Failed to load alerts:', e); }
}

function renderAlertSummary(summary) {
    document.getElementById('alert-summary-cards').innerHTML = `
        <div class="summary-card summary-total">
            <div class="summary-card-value">${summary.total_rules}</div>
            <div class="summary-card-label">Total Rules</div>
        </div>
        <div class="summary-card summary-critical">
            <div class="summary-card-value">${summary.critical}</div>
            <div class="summary-card-label">Critical</div>
        </div>
        <div class="summary-card summary-warning">
            <div class="summary-card-value">${summary.warning}</div>
            <div class="summary-card-label">Warning</div>
        </div>
        <div class="summary-card summary-info">
            <div class="summary-card-value">${summary.info}</div>
            <div class="summary-card-label">Info</div>
        </div>
    `;
}

function renderAlertsList(alerts) {
    const list = document.getElementById('alerts-list');
    if (alerts.length === 0) {
        list.innerHTML = '<div class="no-alerts">✅ No active alerts — all systems normal</div>';
        return;
    }
    list.innerHTML = alerts.map(a => `
        <div class="alert-item severity-${a.severity}">
            <span class="alert-sev alert-sev-${a.severity}">${a.severity}</span>
            <span class="alert-service">${a.service}</span>
            <span class="alert-msg">${escapeHtml(a.message)}</span>
            <span class="alert-time">${formatTimestamp(a.timestamp)}</span>
        </div>
    `).join('');
}

function renderRulesList(rules) {
    document.getElementById('rules-list').innerHTML = rules.map(r => {
        const condStr = formatCondition(r.condition);
        return `
            <div class="rule-item">
                <span class="rule-name">${r.name}</span>
                <span class="rule-target">${r.service === '*' ? 'All' : r.service}</span>
                <span class="rule-condition">${condStr}</span>
                <span class="rule-sev alert-sev-${r.severity}">${r.severity}</span>
                <span class="rule-status ${r.enabled ? 'rule-enabled' : 'rule-disabled'}">${r.enabled ? '●' : '○'}</span>
            </div>
        `;
    }).join('');
}

function formatCondition(cond) {
    if (cond.CpuAbove !== undefined) return `CPU > ${cond.CpuAbove}%`;
    if (cond.MemoryAbove !== undefined) return `MEM > ${cond.MemoryAbove}%`;
    if (cond === 'ContainerDown') return 'Container not running';
    if (cond.RestartLoop !== undefined) return `Restarts > ${cond.RestartLoop}`;
    return JSON.stringify(cond);
}

// ═══════════════════════════════════════
// Toast
// ═══════════════════════════════════════

function showToast(message, type = 'info') {
    const existing = document.querySelector('.toast');
    if (existing) existing.remove();
    const toast = document.createElement('div');
    toast.className = `toast toast-${type}`;
    toast.textContent = message;
    document.body.appendChild(toast);
    requestAnimationFrame(() => { toast.classList.add('visible'); });
    setTimeout(() => { toast.classList.remove('visible'); setTimeout(() => toast.remove(), 300); }, 3000);
}

// ═══════════════════════════════════════
// Utilities
// ═══════════════════════════════════════

function updateTimestamp() {
    document.getElementById('last-update').textContent = `Updated ${new Date().toLocaleTimeString()}`;
}

function formatTimestamp(ts) {
    try {
        const d = new Date(ts);
        if (isNaN(d.getTime())) return ts;
        return d.toLocaleTimeString(undefined, { hour12: false }) + '.' + String(d.getMilliseconds()).padStart(3, '0');
    } catch { return ts; }
}

function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
}

function truncate(str, max) {
    return str.length > max ? str.substring(0, max) + '…' : str;
}
