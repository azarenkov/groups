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
};
CONFIG

mkdir -p /tmp/rift/link-preview.d
rm -f /tmp/rift/link-preview.d/*.conf
upstream="${RIFT_LINK_PREVIEW_UPSTREAM:-}"
if [ -n "$upstream" ]; then
    if printf '%s' "$upstream" | grep -Eq '^https?://[A-Za-z0-9.-]+(:[0-9]+)?$'; then
        cat > /tmp/rift/link-preview.d/upstream.conf <<PREVIEW
if (\$rift_link_preview_bot) {
    rewrite ^/u/([^/]+)/?\$ /api/v1/link-previews/users/\$1 break;
    proxy_pass $upstream;
}
PREVIEW
    else
        echo "RIFT_LINK_PREVIEW_UPSTREAM is not a plain http(s) origin; link previews are off" >&2
    fi
fi
