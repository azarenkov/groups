#!/usr/bin/env bash
set -euo pipefail

TAG="${1:?usage: notes.sh <tag>}"
PREVIOUS="$(git describe --tags --match 'v[0-9]*' --abbrev=0 "${TAG}^" 2>/dev/null || true)"
RANGE="${TAG}"
[ -n "$PREVIOUS" ] && RANGE="${PREVIOUS}..${TAG}"

subjects="$(git log --no-merges --pretty=%s "$RANGE" \
    | grep -vE '^chore: release v[0-9]' \
    | grep -vE '^chore\(deploy\): staging on v[0-9]' || true)"

matched="$(mktemp)"
trap 'rm -f "$matched"' EXIT

render() {
    printf '### %s\n\n' "$1"
    printf '%s\n' "$2" | sed -E 's/^[a-z]+(\([^)]*\))?(!)?: //' | sed 's/^/- /'
    printf '\n'
}

section() {
    local body
    body="$(printf '%s\n' "$subjects" | grep -E "$2" || true)"
    [ -z "$body" ] && return 0
    printf '%s\n' "$body" >> "$matched"
    render "$1" "$body"
}

section "Новое" '^feat(\(|!?:)'
section "Исправления" '^fix(\(|!?:)'
section "Сборка и инфраструктура" '^(ci|build|deploy)(\(|!?:)|^chore\(deploy\)'

rest="$(printf '%s\n' "$subjects" | grep -vxF -f "$matched" || true)"
[ -n "$rest" ] && render "Прочее" "$rest"

if [ -n "$PREVIOUS" ]; then
    printf 'Полный список изменений: https://github.com/rift-space/rift/compare/%s...%s\n' "$PREVIOUS" "$TAG"
fi
