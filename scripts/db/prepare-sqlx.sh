#!/usr/bin/env bash
set -euo pipefail

ADMIN_URL="${SQLX_PREPARE_ADMIN_URL:-postgres://postgres:postgres@localhost:5432/postgres}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BASE_URL="${ADMIN_URL%/*}"
SERVICES=(identity media messaging voice realtime)

database_url() {
    printf '%s/sqlx_prepare_%s?options=-c%%20search_path%%3D%s%%2Cpublic' "$BASE_URL" "$1" "$1"
}

cleanup() {
    for service in "${SERVICES[@]}"; do
        sqlx database drop -y --database-url "$(database_url "$service")" >/dev/null 2>&1 || true
    done
}
trap cleanup EXIT

for service in "${SERVICES[@]}"; do
    sqlx database drop -y --database-url "$(database_url "$service")" >/dev/null 2>&1 || true
    sqlx database create --database-url "$(database_url "$service")"
    sqlx migrate run --source "$ROOT/backend/services/${service}/migrations" --database-url "$(database_url "$service")"
done

prepare() {
    local crate_dir="$1" service="$2"
    rm -rf "${ROOT:?}/$crate_dir/.sqlx"
    (cd "$ROOT/$crate_dir" && DATABASE_URL="$(database_url "$service")" SQLX_OFFLINE=false cargo sqlx prepare)
}

prepare backend/libs/infrastructure-common messaging
for service in "${SERVICES[@]}"; do
    prepare "backend/services/${service}/infrastructure" "$service"
done

for service in "${SERVICES[@]}"; do
    for shared in "$ROOT"/backend/libs/infrastructure-common/.sqlx/*.json; do
        rm -f "$ROOT/backend/services/${service}/infrastructure/.sqlx/$(basename "$shared")"
    done
done
