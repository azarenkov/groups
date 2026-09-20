#!/usr/bin/env bash
set -euo pipefail

NAMESPACE="${NAMESPACE:-rift}"
APP_HOST="${APP_HOST:-rift.localtest.me}"
LIVEKIT_HOST="${LIVEKIT_HOST:-livekit.localtest.me}"
SCHEME="${SCHEME:-https}"
RESOLVE="${RESOLVE:-127.0.0.1}"

echo "==> waiting for deployments in $NAMESPACE"
kubectl -n "$NAMESPACE" wait --for=condition=Available deployment --all --timeout=600s

curl_app() {
    curl -sk --resolve "$APP_HOST:443:$RESOLVE" --resolve "$APP_HOST:80:$RESOLVE" "$@"
}

echo "==> gateway health"
test "$(curl_app -o /dev/null -w '%{http_code}' "$SCHEME://$APP_HOST/healthz")" = "200"

echo "==> web shell"
curl_app "$SCHEME://$APP_HOST/" | grep -q 'id="root"'
curl_app "$SCHEME://$APP_HOST/config.js" | grep -q '__RIFT_CONFIG__'

echo "==> api answers through the gateway"
code="$(curl_app -o /dev/null -w '%{http_code}' -X POST -H 'Content-Type: application/json' -d '{}' "$SCHEME://$APP_HOST/api/v1/auth/login")"
case "$code" in
    400|401|422) ;;
    *) echo "unexpected status $code from /api/v1/auth/login"; exit 1 ;;
esac

echo "==> websocket endpoint is routed"
code="$(curl_app -o /dev/null -w '%{http_code}' "$SCHEME://$APP_HOST/ws")"
case "$code" in
    400|401|426) ;;
    *) echo "unexpected status $code from /ws"; exit 1 ;;
esac

echo "==> livekit answers through the gateway"
test "$(curl -sk --resolve "$LIVEKIT_HOST:443:$RESOLVE" --resolve "$LIVEKIT_HOST:80:$RESOLVE" -o /dev/null -w '%{http_code}' "$SCHEME://$LIVEKIT_HOST/")" = "200"

echo "==> every service is ready"
for deploy in $(kubectl -n "$NAMESPACE" get deployment -o jsonpath='{.items[*].metadata.name}'); do
    ready="$(kubectl -n "$NAMESPACE" get deployment "$deploy" -o jsonpath='{.status.readyReplicas}')"
    echo "   $deploy: ${ready:-0} ready"
    test "${ready:-0}" -ge 1
done

echo "smoke ok"
