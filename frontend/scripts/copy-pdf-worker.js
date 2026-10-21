const fs = require("fs");
const path = require("path");

const source = require.resolve("pdfjs-dist/legacy/build/pdf.worker.min.js");
const targetDir = path.join(__dirname, "..", "public", "vendor");
fs.mkdirSync(targetDir, { recursive: true });
fs.copyFileSync(source, path.join(targetDir, "pdf.worker.min.js"));
