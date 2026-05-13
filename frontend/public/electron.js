const { app, BrowserWindow, ipcMain, nativeImage } = require('electron');
const path = require('path');
const isDev = require('electron-is-dev');

app.setName('Rift');

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

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1280,
    height: 720,
    minWidth: 800,
    minHeight: 600,
    backgroundColor: '#0d0d0f',
    titleBarStyle: 'hiddenInset',
    frame: false,
    icon: appIcon,
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

  if (isDev) {
    mainWindow.webContents.openDevTools();
  }

  mainWindow.on('closed', () => {
    mainWindow = null;
  });
}

app.on('ready', createWindow);

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

app.on('activate', () => {
  if (mainWindow === null) {
    createWindow();
  }
});

// Window controls for frameless window
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
