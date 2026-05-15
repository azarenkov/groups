const fs = require("fs");
const path = require("path");

const SRC = path.join(__dirname, "..", "node_modules", "onnxruntime-web", "dist");
const DEST = path.join(__dirname, "..", "public", "ort");

if (!fs.existsSync(SRC)) {
  console.warn("[copy-ort-wasm] source not found, skipping:", SRC);
  process.exit(0);
}

fs.mkdirSync(DEST, { recursive: true });

const files = fs
  .readdirSync(SRC)
  .filter((f) => f.endsWith(".wasm") || f.endsWith(".mjs"));

for (const f of files) {
  fs.copyFileSync(path.join(SRC, f), path.join(DEST, f));
}

console.log(`[copy-ort-wasm] copied ${files.length} files to public/ort/`);
