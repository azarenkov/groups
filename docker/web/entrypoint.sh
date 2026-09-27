#!/bin/sh
set -eu

escape() {
    printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

mkdir -p /tmp/rift
cat > /tmp/rift/config.js <<CONFIG
window.__RIFT_CONFIG__ = {
  apiUrl: "$(escape "${RIFT_API_URL:-/api}")",
  wsUrl: "$(escape "${RIFT_WS_URL:-/ws}")",
  appUrl: "$(escape "${RIFT_APP_URL:-}")",
  googleClientId: "$(escape "${RIFT_GOOGLE_CLIENT_ID:-}")"
};
CONFIG
