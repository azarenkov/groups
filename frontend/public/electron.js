const { app, BrowserWindow, Menu, ipcMain, nativeImage, safeStorage, session, shell } = require('electron');
const fs = require('fs');
const http = require('http');
const crypto = require('crypto');
const path = require('path');
const isDev = require('electron-is-dev');

const GOOGLE_AUTH_ENDPOINT = 'https://accounts.google.com/o/oauth2/v2/auth';
const GOOGLE_TOKEN_ENDPOINT = 'https://oauth2.googleapis.com/token';
const GOOGLE_OAUTH_TIMEOUT_MS = 5 * 60 * 1000;

function base64Url(buffer) {
  return buffer
    .toString('base64')
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '');
}

if (isDev) {
  process.env.ELECTRON_DISABLE_SECURITY_WARNINGS = 'true';
}

app.setName('Rift');

const UPDATE_CHECK_INTERVAL_MS = 60 * 60 * 1000;
const UPDATE_BOOT_TIMEOUT_MS = 8000;

let autoUpdater = null;
if (!isDev) {
  try {
    autoUpdater = require('electron-updater').autoUpdater;
    autoUpdater.autoDownload = true;
    autoUpdater.autoInstallOnAppQuit = true;
  } catch (e) {
    console.error('[updater] failed to load electron-updater:', e);
  }
}

const iconPath = path.join(
  __dirname,
  'icons',
  process.platform === 'win32' ? 'rift-icon-256-padded.png' : 'rift-icon-1024-padded.png',
);
const appIcon = nativeImage.createFromPath(iconPath);

if (process.platform === 'darwin' && app.dock && !appIcon.isEmpty()) {
  app.dock.setIcon(appIcon);
}

let mainWindow;
let splashWindow;

const ZOOM_MIN = 0.7;
const ZOOM_MAX = 1.5;
const ZOOM_STEP = 0.1;

function createSplash() {
  splashWindow = new BrowserWindow({
    width: 360,
    height: 280,
    resizable: false,
    movable: true,
    minimizable: false,
    maximizable: false,
    fullscreenable: false,
    frame: false,
    transparent: false,
    backgroundColor: '#0d0d0f',
    icon: appIcon,
    webPreferences: {
      nodeIntegration: true,
      contextIsolation: false,
    },
  });
  splashWindow.loadFile(path.join(__dirname, 'splash.html'));
  splashWindow.on('closed', () => {
    splashWindow = null;
  });
}

function setSplashStatus(text) {
  if (splashWindow && !splashWindow.isDestroyed()) {
    splashWindow.webContents.send('updater:status', text);
  }
}

function setSplashProgress(percent) {
  if (splashWindow && !splashWindow.isDestroyed()) {
    splashWindow.webContents.send('updater:progress', percent);
  }
}

function closeSplash() {
  if (splashWindow && !splashWindow.isDestroyed()) {
    splashWindow.destroy();
  }
  splashWindow = null;
}

function createMainWindow() {
  mainWindow = new BrowserWindow({
    width: 1280,
    height: 720,
    minWidth: 800,
    minHeight: 600,
    backgroundColor: '#0d0d0f',
    titleBarStyle: 'hiddenInset',
    frame: false,
    icon: appIcon,
    show: isDev,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      enableRemoteModule: false,
      preload: path.join(__dirname, 'preload.js'),
    },
  });

  const startURL = isDev
    ? 'http://localhost:3000'
    : `file://${path.join(__dirname, '../build/index.html')}`;

  mainWindow.loadURL(startURL);

  mainWindow.once('ready-to-show', () => {
    closeSplash();
    mainWindow.show();
  });

  if (isDev || process.env.RIFT_DEVTOOLS === '1') {
    mainWindow.webContents.once('did-finish-load', () => {
      mainWindow.webContents.openDevTools({ mode: 'detach' });
    });
  }

  mainWindow.webContents.setVisualZoomLevelLimits(1, 1);

  const clampZoom = () => {
    const current = mainWindow.webContents.getZoomFactor();
    if (current > ZOOM_MAX + 0.001) {
      mainWindow.webContents.setZoomFactor(ZOOM_MAX);
    } else if (current < ZOOM_MIN - 0.001) {
      mainWindow.webContents.setZoomFactor(ZOOM_MIN);
    }
  };

  mainWindow.webContents.on('zoom-changed', () => {
    clampZoom();
  });

  mainWindow.webContents.on('before-input-event', (event, input) => {
    if (input.type !== 'keyDown') return;
    const mod = input.meta || input.control;
    if (!mod) return;
    const key = input.key;
    if (key === '=' || key === '+') {
      const next = Math.min(
        ZOOM_MAX,
        +(mainWindow.webContents.getZoomFactor() + ZOOM_STEP).toFixed(2),
      );
      mainWindow.webContents.setZoomFactor(next);
      event.preventDefault();
    } else if (key === '-') {
      const next = Math.max(
        ZOOM_MIN,
        +(mainWindow.webContents.getZoomFactor() - ZOOM_STEP).toFixed(2),
      );
      mainWindow.webContents.setZoomFactor(next);
      event.preventDefault();
    } else if (key === '0') {
      mainWindow.webContents.setZoomFactor(1.0);
      event.preventDefault();
    }
  });

  mainWindow.on('closed', () => {
    mainWindow = null;
  });
}

function runBootUpdateThenLaunch() {
  let resolved = false;

  const proceed = () => {
    if (resolved) return;
    resolved = true;
    createMainWindow();
  };

  const installNow = () => {
    if (resolved) return;
    resolved = true;
    setSplashStatus('Installing update…');
    setTimeout(() => autoUpdater.quitAndInstall(true, true), 250);
  };

  autoUpdater.on('error', (err) => {
    console.error('[updater] error:', err);
    proceed();
  });
  autoUpdater.on('update-not-available', () => {
    proceed();
  });
  autoUpdater.on('update-available', (info) => {
    setSplashStatus(`Downloading update ${info.version}…`);
  });
  autoUpdater.on('download-progress', (p) => {
    setSplashProgress(p.percent);
  });
  autoUpdater.on('update-downloaded', () => {
    installNow();
  });

  setTimeout(() => {
    if (!resolved) {
      console.warn('[updater] boot check timed out, launching app');
      proceed();
    }
  }, UPDATE_BOOT_TIMEOUT_MS);

  autoUpdater.checkForUpdates().catch((err) => {
    console.error('[updater] checkForUpdates failed:', err);
    proceed();
  });
}

app.on('ready', () => {
  Menu.setApplicationMenu(null);

  session.defaultSession.webRequest.onHeadersReceived((details, callback) => {
    const headers = { ...details.responseHeaders };
    headers['Cross-Origin-Opener-Policy'] = ['same-origin'];
    headers['Cross-Origin-Embedder-Policy'] = ['credentialless'];
    callback({ responseHeaders: headers });
  });

  if (autoUpdater) {
    createSplash();
    runBootUpdateThenLaunch();
    setInterval(() => {
      autoUpdater.checkForUpdates().catch((err) => {
        console.error('[updater] periodic check failed:', err);
      });
    }, UPDATE_CHECK_INTERVAL_MS);
  } else {
    createMainWindow();
  }
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

app.on('activate', () => {
  if (mainWindow === null) {
    createMainWindow();
  }
});

ipcMain.on('window-minimize', () => {
  if (mainWindow) mainWindow.minimize();
});

ipcMain.on('window-maximize', () => {
  if (mainWindow) {
    if (mainWindow.isMaximized()) {
      mainWindow.unmaximize();
    } else {
      mainWindow.maximize();
    }
  }
});

ipcMain.on('window-close', () => {
  if (mainWindow) mainWindow.close();
});

function secureFilePath() {
  return path.join(app.getPath('userData'), 'auth.bin');
}

ipcMain.handle('secure:get', () => {
  try {
    const file = secureFilePath();
    if (!fs.existsSync(file)) return null;
    if (!safeStorage.isEncryptionAvailable()) return null;
    const encrypted = fs.readFileSync(file);
    return safeStorage.decryptString(encrypted);
  } catch (e) {
    console.error('[secure:get] failed:', e);
    return null;
  }
});

ipcMain.handle('secure:set', (_event, value) => {
  try {
    if (typeof value !== 'string') return false;
    if (!safeStorage.isEncryptionAvailable()) return false;
    const encrypted = safeStorage.encryptString(value);
    fs.writeFileSync(secureFilePath(), encrypted, { mode: 0o600 });
    return true;
  } catch (e) {
    console.error('[secure:set] failed:', e);
    return false;
  }
});

ipcMain.handle('oauth:google', async () => {
  const clientId = process.env.GOOGLE_DESKTOP_CLIENT_ID;
  const clientSecret = process.env.GOOGLE_DESKTOP_CLIENT_SECRET;
  if (!clientId) {
    throw new Error('GOOGLE_DESKTOP_CLIENT_ID is not configured');
  }

  const codeVerifier = base64Url(crypto.randomBytes(32));
  const codeChallenge = base64Url(
    crypto.createHash('sha256').update(codeVerifier).digest(),
  );
  const state = base64Url(crypto.randomBytes(16));

  return await new Promise((resolve, reject) => {
    let settled = false;
    let timer = null;

    const finish = (fn, value) => {
      if (settled) return;
      settled = true;
      if (timer) clearTimeout(timer);
      try {
        server.close();
      } catch (_e) {
        void _e;
      }
      fn(value);
    };

    const server = http.createServer(async (req, res) => {
      let reqUrl;
      try {
        reqUrl = new URL(req.url, 'http://127.0.0.1');
      } catch (_e) {
        res.statusCode = 400;
        res.end();
        return;
      }
      if (reqUrl.pathname !== '/') {
        res.statusCode = 404;
        res.end();
        return;
      }

      res.setHeader('Content-Type', 'text/html');
      res.end(
        '<!doctype html><html><body style="font-family:sans-serif;background:#0d0d0f;color:#fff;display:flex;align-items:center;justify-content:center;height:100vh;margin:0"><p>You can close this window and return to Rift.</p></body></html>',
      );

      const error = reqUrl.searchParams.get('error');
      const code = reqUrl.searchParams.get('code');
      const returnedState = reqUrl.searchParams.get('state');

      if (error) {
        finish(reject, new Error(error));
        return;
      }
      if (!code || returnedState !== state) {
        finish(reject, new Error('invalid oauth response'));
        return;
      }

      try {
        const redirectUri = `http://127.0.0.1:${server.address().port}`;
        const body = new URLSearchParams({
          code,
          client_id: clientId,
          redirect_uri: redirectUri,
          grant_type: 'authorization_code',
          code_verifier: codeVerifier,
        });
        if (clientSecret) body.set('client_secret', clientSecret);

        const tokenResponse = await fetch(GOOGLE_TOKEN_ENDPOINT, {
          method: 'POST',
          headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
          body: body.toString(),
        });
        if (!tokenResponse.ok) {
          finish(reject, new Error('token exchange failed'));
          return;
        }
        const json = await tokenResponse.json();
        if (!json.id_token) {
          finish(reject, new Error('no id_token in token response'));
          return;
        }
        finish(resolve, json.id_token);
      } catch (e) {
        finish(reject, e);
      }
    });

    server.on('error', (e) => finish(reject, e));

    server.listen(0, '127.0.0.1', () => {
      const redirectUri = `http://127.0.0.1:${server.address().port}`;
      const authUrl = new URL(GOOGLE_AUTH_ENDPOINT);
      authUrl.searchParams.set('client_id', clientId);
      authUrl.searchParams.set('redirect_uri', redirectUri);
      authUrl.searchParams.set('response_type', 'code');
      authUrl.searchParams.set('scope', 'openid email profile');
      authUrl.searchParams.set('code_challenge', codeChallenge);
      authUrl.searchParams.set('code_challenge_method', 'S256');
      authUrl.searchParams.set('state', state);
      shell.openExternal(authUrl.toString());
    });

    timer = setTimeout(() => finish(reject, new Error('oauth timed out')), GOOGLE_OAUTH_TIMEOUT_MS);
  });
});

ipcMain.handle('secure:clear', () => {
  try {
    const file = secureFilePath();
    if (fs.existsSync(file)) fs.unlinkSync(file);
    return true;
  } catch (e) {
    console.error('[secure:clear] failed:', e);
    return false;
  }
});
