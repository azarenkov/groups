const fs = require('fs');
const path = require('path');

const buildDir = path.join(__dirname, '..', 'build');
const config = {
  googleDesktopClientId: process.env.GOOGLE_DESKTOP_CLIENT_ID || '',
  googleDesktopClientSecret: process.env.GOOGLE_DESKTOP_CLIENT_SECRET || '',
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
