#!/usr/bin/env bash
set -euo pipefail

TAG="${1:?usage: notes.sh <tag>}"
PREVIOUS="$(git tag --list 'v[0-9]*' --sort=-version:refname | grep -v "^${TAG}$" | head -n1 || true)"
RANGE="${TAG}"
[ -n "$PREVIOUS" ] && RANGE="${PREVIOUS}..${TAG}"

subjects="$(git log --no-merges --pretty=%s "$RANGE" \
    | grep -vE '^chore: release v[0-9]' \
    | grep -vE '^chore\(deploy\): staging on v[0-9]')"

section() {
    local title="$1" pattern="$2" body
    body="$(printf '%s\n' "$subjects" | grep -E "$pattern" | sed -E 's/^[a-z]+(\([^)]*\))?(!)?: //' || true)"
    [ -z "$body" ] && return 0
    printf '### %s\n\n' "$title"
    printf '%s\n' "$body" | sed 's/^/- /'
    printf '\n'
}

section "Новое" '^feat(\(|!?:)'
section "Исправления" '^fix(\(|!?:)'
section "Сборка и инфраструктура" '^(ci|build|deploy|chore\(deploy\))(\(|!?:)'
section "Прочее" '^(refactor|perf|style|test|docs|chore)(\(|!?:)'

if [ -n "$PREVIOUS" ]; then
    printf 'Полный список изменений: https://github.com/rift-space/rift/compare/%s...%s\n' "$PREVIOUS" "$TAG"
fi
