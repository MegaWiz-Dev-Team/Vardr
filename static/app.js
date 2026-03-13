// ═══════════════════════════════════════
// 🛡️ Várðr — Dashboard App
// ═══════════════════════════════════════

const API = '';
let currentTab = 'services';
let logStream = null;
let refreshInterval = null;

// ── Init ──
document.addEventListener('DOMContentLoaded', () => {
    setupTabs();
    loadServices();
    startAutoRefresh();
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
        });
    });

    document.getElementById('log-fetch').addEventListener('click', fetchLogs);
    document.getElementById('log-stream-toggle').addEventListener('click', toggleStream);
}

// ── Auto Refresh ──
function startAutoRefresh() {
    refreshInterval = setInterval(() => {
        if (currentTab === 'services') loadServices();
        if (currentTab === 'metrics') loadMetrics();
    }, 10000);
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
    } catch (e) {
        console.error('Failed to load services:', e);
    }
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
            <div class="service-card ${stateClass}" onclick="viewServiceLogs('${s.name}')">
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
    if (status.toLowerCase().includes('healthy')) {
        return '<span class="status-badge status-healthy">Healthy</span>';
    }
    if (status.toLowerCase().includes('unhealthy')) {
        return '<span class="status-badge status-stopped">Unhealthy</span>';
    }
    if (state === 'running') {
        return '<span class="status-badge status-running">Running</span>';
    }
    if (state === 'restarting') {
        return '<span class="status-badge status-restarting">Restarting</span>';
    }
    if (state === 'exited') {
        return '<span class="status-badge status-stopped">Stopped</span>';
    }
    return `<span class="status-badge status-running">${state}</span>`;
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
    } catch (e) {
        console.error('Failed to load service list:', e);
    }
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
    } catch (e) {
        console.error('Failed to fetch logs:', e);
    }
}

function renderLogs(logs) {
    const viewer = document.getElementById('log-viewer');
    if (logs.length === 0) {
        viewer.innerHTML = '<div class="log-placeholder">No logs found</div>';
        return;
    }

    viewer.innerHTML = logs.map(l => `
        <div class="log-line">
            <span class="log-ts">${formatTimestamp(l.timestamp)}</span>
            <span class="log-level log-level-${l.level}">${l.level}</span>
            <span class="log-msg">${escapeHtml(l.message)}</span>
        </div>
    `).join('');

    if (document.getElementById('log-autoscroll').checked) {
        viewer.scrollTop = viewer.scrollHeight;
    }
}

function viewServiceLogs(serviceName) {
    // Switch to logs tab and pre-select service
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

// ── SSE Stream ──
function toggleStream() {
    const btn = document.getElementById('log-stream-toggle');
    if (logStream) {
        logStream.close();
        logStream = null;
        btn.textContent = '▶ Stream';
        btn.classList.remove('active');
        return;
    }

    const service = document.getElementById('log-service').value;
    if (!service) return;

    btn.textContent = '⏹ Stop';
    btn.classList.add('active');

    logStream = new EventSource(`${API}/api/logs/stream/${service}`);
    logStream.onmessage = (event) => {
        try {
            const entry = JSON.parse(event.data);
            appendLogEntry(entry);
        } catch (e) {
            console.error('Failed to parse SSE:', e);
        }
    };

    logStream.onerror = () => {
        logStream.close();
        logStream = null;
        btn.textContent = '▶ Stream';
        btn.classList.remove('active');
    };
}

function appendLogEntry(entry) {
    const viewer = document.getElementById('log-viewer');
    // Remove placeholder if present
    const placeholder = viewer.querySelector('.log-placeholder');
    if (placeholder) placeholder.remove();

    const div = document.createElement('div');
    div.className = 'log-line';
    div.innerHTML = `
        <span class="log-ts">${formatTimestamp(entry.timestamp)}</span>
        <span class="log-level log-level-${entry.level}">${entry.level}</span>
        <span class="log-msg">${escapeHtml(entry.message)}</span>
    `;
    viewer.appendChild(div);

    // Keep max 1000 lines
    while (viewer.children.length > 1000) {
        viewer.removeChild(viewer.firstChild);
    }

    if (document.getElementById('log-autoscroll').checked) {
        viewer.scrollTop = viewer.scrollHeight;
    }
}

// ═══════════════════════════════════════
// Metrics
// ═══════════════════════════════════════

async function loadMetrics() {
    try {
        const resp = await fetch(`${API}/api/metrics`);
        const metrics = await resp.json();
        renderMetrics(metrics);
    } catch (e) {
        console.error('Failed to load metrics:', e);
    }
}

function renderMetrics(metrics) {
    const grid = document.getElementById('metrics-grid');
    if (metrics.length === 0) {
        grid.innerHTML = '<div class="loading">No metrics available</div>';
        return;
    }

    grid.innerHTML = metrics.map(m => `
        <div class="metric-card">
            <div class="metric-header">
                <span class="metric-name">${m.service}</span>
                <span class="metric-pids">${m.pids} PIDs</span>
            </div>
            <div class="metric-bars">
                <div class="metric-row">
                    <div class="metric-label">
                        <span>CPU</span>
                        <span>${m.cpu_percent.toFixed(2)}%</span>
                    </div>
                    <div class="metric-bar-bg">
                        <div class="metric-bar metric-bar-cpu" style="width: ${Math.min(m.cpu_percent, 100)}%"></div>
                    </div>
                </div>
                <div class="metric-row">
                    <div class="metric-label">
                        <span>Memory</span>
                        <span>${m.memory_usage_mb.toFixed(1)} / ${m.memory_limit_mb.toFixed(0)} MB (${m.memory_percent.toFixed(1)}%)</span>
                    </div>
                    <div class="metric-bar-bg">
                        <div class="metric-bar metric-bar-mem" style="width: ${Math.min(m.memory_percent, 100)}%"></div>
                    </div>
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
// Utilities
// ═══════════════════════════════════════

function updateTimestamp() {
    const el = document.getElementById('last-update');
    el.textContent = `Updated ${new Date().toLocaleTimeString()}`;
}

function formatTimestamp(ts) {
    try {
        const d = new Date(ts);
        if (isNaN(d.getTime())) return ts;
        return d.toLocaleTimeString(undefined, { hour12: false }) + '.' + String(d.getMilliseconds()).padStart(3, '0');
    } catch {
        return ts;
    }
}

function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
}

function truncate(str, max) {
    return str.length > max ? str.substring(0, max) + '…' : str;
}
