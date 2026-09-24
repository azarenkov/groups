#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CRATE="$ROOT/frontend/wasm/deepfilter"
OUT="$ROOT/frontend/src/shared/lib/deepfilter"
IMAGE="${DEEPFILTER_BUILD_IMAGE:-rust:1.97-slim-bookworm}"

docker run --rm \
  -v "$CRATE:/crate" \
  -v rift-deepfilter-cargo:/usr/local/cargo/registry \
  -w /crate \
  "$IMAGE" \
  bash -euo pipefail -c '
    apt-get update -qq >/dev/null && apt-get install -y -qq git pkg-config >/dev/null
    rustup target add wasm32-unknown-unknown >/dev/null
    [ -f Cargo.lock ] || cargo generate-lockfile -q
    version="$(cargo pkgid -q wasm-bindgen | sed "s/.*[@#]//")"
    cargo install -q wasm-bindgen-cli --version "$version" --locked
    RUSTFLAGS="-C target-feature=+simd128" cargo build -q --release --target wasm32-unknown-unknown
    rm -rf pkg
    wasm-bindgen --target web --no-typescript --out-dir pkg --out-name deepfilter \
      target/wasm32-unknown-unknown/release/rift_deepfilter.wasm
    rm -rf target
  '

mkdir -p "$OUT"
cp "$CRATE/pkg/deepfilter_bg.wasm" "$OUT/deepfilter_bg.wasm"
npx --prefix "$ROOT/frontend" terser "$CRATE/pkg/deepfilter.js" \
  --module --comments false --format beautify=true,indent_level=2 \
  -o "$OUT/deepfilter.js"
rm -rf "$CRATE/pkg"
ls -la "$OUT"
