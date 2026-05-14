const { app, BrowserWindow, Menu, ipcMain, nativeImage, safeStorage } = require('electron');
const fs = require('fs');
const path = require('path');
const isDev = require('electron-is-dev');

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

  if (isDev) {
    mainWindow.webContents.openDevTools();
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
