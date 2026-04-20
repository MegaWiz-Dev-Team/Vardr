#!/usr/bin/env bash
set -euo pipefail

# ============================================
# 🛡️ Várðr Native Agent Installer
# ============================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLIST_NAME="com.asgard.vardr-agent"
LAUNCHD_DIR="$HOME/Library/LaunchAgents"
PLIST_PATH="$LAUNCHD_DIR/$PLIST_NAME.plist"
LOG_DIR="$SCRIPT_DIR/logs"

# Ensure we are compiling the agent
echo "🛠️ Compiling Várðr Agent..."
cd "$SCRIPT_DIR/agent"
cargo build --release

mkdir -p "$LOG_DIR"
mkdir -p "$LAUNCHD_DIR"

# Generate Plist
echo "📦 Generating launchd configuration ($PLIST_NAME)..."
cat <<EOF > "$PLIST_PATH"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>$PLIST_NAME</string>

    <key>ProgramArguments</key>
    <array>
        <string>$SCRIPT_DIR/agent/target/release/agent</string>
    </array>

    <key>WorkingDirectory</key>
    <string>$SCRIPT_DIR/agent</string>

    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>

    <key>ThrottleInterval</key>
    <integer>5</integer>

    <key>RunAtLoad</key>
    <true/>

    <key>StandardOutPath</key>
    <string>$LOG_DIR/agent-stdout.log</string>
    <key>StandardErrorPath</key>
    <string>$LOG_DIR/agent-stderr.log</string>
</dict>
</plist>
EOF

# Restart Agent
echo "🔄 Reloading Várðr Agent service..."
launchctl unload "$PLIST_PATH" 2>/dev/null || true
launchctl load "$PLIST_PATH"
launchctl start "$PLIST_NAME"

echo "✅ Várðr Agent is now running natively on Port 9091!"
echo "   It successfully monitors macOS Host and bridges it to K3s."
