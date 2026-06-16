const fs = require('fs');
const path = require('path');

function readEnvFile(file) {
  const out = {};
  let content;
  try {
    if (!fs.existsSync(file)) return out;
    content = fs.readFileSync(file, 'utf8');
  } catch (_e) {
    return out;
  }
  for (const rawLine of content.split('\n')) {
    const line = rawLine.trim();
    if (!line || line.startsWith('#')) continue;
    const eq = line.indexOf('=');
    if (eq === -1) continue;
    const key = line.slice(0, eq).trim();
    if (!key) continue;
    let value = line.slice(eq + 1).trim();
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    out[key] = value;
  }
  return out;
}

function resolve(key) {
  if (process.env[key] !== undefined && process.env[key] !== '') {
    return process.env[key];
  }
  const candidates = [
    path.join(process.cwd(), '.env'),
    path.join(__dirname, '..', '.env'),
    path.join(__dirname, '..', '..', '.env'),
  ];
  for (const file of candidates) {
    const parsed = readEnvFile(file);
    if (parsed[key]) return parsed[key];
  }
  return '';
}

const buildDir = path.join(__dirname, '..', 'build');
const config = {
  googleDesktopClientId: resolve('GOOGLE_DESKTOP_CLIENT_ID'),
  googleDesktopClientSecret: resolve('GOOGLE_DESKTOP_CLIENT_SECRET'),
};

fs.mkdirSync(buildDir, { recursive: true });
fs.writeFileSync(
  path.join(buildDir, 'desktop-config.json'),
  JSON.stringify(config),
);

console.log(
  '[write-desktop-config] desktop-config.json written (google client id set:',
  !!config.googleDesktopClientId,
  ')',
);
