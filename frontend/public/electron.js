const { app, BrowserWindow, Menu, desktopCapturer, ipcMain, nativeImage, safeStorage, session, shell } = require('electron');
const fs = require('fs');
const { exec } = require('child_process');
const path = require('path');
const isDev = require('electron-is-dev');
const { pathToFileURL } = require('url');

function loadDotEnv() {
  const candidates = [
    path.join(process.cwd(), '.env'),
    path.join(__dirname, '..', '.env'),
    path.join(__dirname, '..', '..', '.env'),
  ];
  for (const file of candidates) {
    let content;
    try {
      if (!fs.existsSync(file)) continue;
      content = fs.readFileSync(file, 'utf8');
    } catch (_e) {
      continue;
    }
    for (const rawLine of content.split('\n')) {
      const line = rawLine.trim();
      if (!line || line.startsWith('#')) continue;
      const eq = line.indexOf('=');
      if (eq === -1) continue;
      const key = line.slice(0, eq).trim();
      if (!key || process.env[key] !== undefined) continue;
      let value = line.slice(eq + 1).trim();
      if (
        (value.startsWith('"') && value.endsWith('"')) ||
        (value.startsWith("'") && value.endsWith("'"))
      ) {
        value = value.slice(1, -1);
      }
      process.env[key] = value;
    }
  }
}

loadDotEnv();

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
let pendingDeepLink = null;

const DEEP_LINK_PROTOCOL = 'rift';

function isInviteDeepLink(value) {
  if (typeof value !== 'string') return false;
  const url = value.trim();
  return /^rift:\/\/join\/[A-Za-z0-9_-]{22,64}\/?$/.test(url)
    || /^rift:\/\/reset-password\/?\?token=[0-9a-f]{32}\.[0-9a-f]{64}$/.test(url)
    || /^rift:\/\/u\/[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\/?$/.test(url)
    || /^rift:\/\/app\/(home|settings\/[a-z]+|dm\/[0-9a-fA-F~-]{36,73}(\/[0-9a-fA-F-]{36})?|c\/[0-9a-fA-F-]{36}\/[0-9a-fA-F-]{36}(\/[0-9a-fA-F-]{36})?)\/?$/.test(url);
}

function deepLinkFromArgv(argv) {
  return (argv || []).find((arg) => isInviteDeepLink(arg)) || null;
}

function deliverDeepLink(url) {
  if (!isInviteDeepLink(url)) return;
  pendingDeepLink = url.trim();
  if (mainWindow && !mainWindow.isDestroyed()) {
    if (mainWindow.isMinimized()) mainWindow.restore();
    mainWindow.show();
    mainWindow.focus();
    if (!mainWindow.webContents.isLoading()) {
      mainWindow.webContents.send('deep-link', pendingDeepLink);
    }
  }
}

const hasSingleInstanceLock = app.requestSingleInstanceLock();
if (!hasSingleInstanceLock) {
  app.quit();
} else {
  app.on('second-instance', (_event, argv) => {
    const url = deepLinkFromArgv(argv);
    if (url) deliverDeepLink(url);
    else if (mainWindow && !mainWindow.isDestroyed()) {
      if (mainWindow.isMinimized()) mainWindow.restore();
      mainWindow.focus();
    }
  });
}

app.on('open-url', (event, url) => {
  event.preventDefault();
  deliverDeepLink(url);
});

pendingDeepLink = deepLinkFromArgv(process.argv);

ipcMain.handle('deep-link:pending', () => {
  const url = pendingDeepLink;
  pendingDeepLink = null;
  return url;
});

ipcMain.on('deep-link:consumed', (_event, url) => {
  if (pendingDeepLink === url) pendingDeepLink = null;
});

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
      nodeIntegration: false,
      contextIsolation: true,
      sandbox: true,
      preload: path.join(__dirname, 'splash-preload.js'),
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

const GAME_CATALOG = [
  { id: 'dota2', name: 'Dota 2', processes: ['dota2.exe', 'dota2', 'dota_osx', 'dota2_osx'] },
  { id: 'cs2', name: 'Counter-Strike 2', processes: ['cs2.exe', 'cs2'] },
  { id: 'deadlock', name: 'Deadlock', processes: ['project8.exe', 'project8'] },
  { id: 'tf2', name: 'Team Fortress 2', processes: ['tf_win64.exe', 'tf_linux64', 'tf_osx64'] },
  { id: 'lol', name: 'League of Legends', processes: ['league of legends.exe', 'league of legends'] },
  { id: 'valorant', name: 'VALORANT', processes: ['valorant-win64-shipping.exe'] },
  { id: 'fortnite', name: 'Fortnite', processes: ['fortniteclient-win64-shipping.exe'] },
  { id: 'apex', name: 'Apex Legends', processes: ['r5apex.exe', 'r5apex_dx12.exe'] },
  { id: 'pubg', name: 'PUBG: Battlegrounds', processes: ['tslgame.exe'] },
  { id: 'warzone', name: 'Call of Duty', processes: ['cod.exe', 'modernwarfare.exe'] },
  { id: 'overwatch2', name: 'Overwatch 2', processes: ['overwatch.exe'] },
  { id: 'marvel-rivals', name: 'Marvel Rivals', processes: ['marvel-win64-shipping.exe'] },
  { id: 'r6', name: 'Rainbow Six Siege', processes: ['rainbowsix.exe', 'rainbowsix_be.exe'] },
  { id: 'rocket-league', name: 'Rocket League', processes: ['rocketleague.exe'] },
  { id: 'gta5', name: 'Grand Theft Auto V', processes: ['gta5.exe', 'gta5_enhanced.exe'] },
  { id: 'rust', name: 'Rust', processes: ['rustclient.exe'] },
  { id: 'tarkov', name: 'Escape from Tarkov', processes: ['escapefromtarkov.exe'] },
  { id: 'wow', name: 'World of Warcraft', processes: ['wow.exe', 'wowclassic.exe', 'world of warcraft'] },
  { id: 'hearthstone', name: 'Hearthstone', processes: ['hearthstone.exe', 'hearthstone'] },
  { id: 'sc2', name: 'StarCraft II', processes: ['sc2_x64.exe', 'sc2'] },
  { id: 'diablo4', name: 'Diablo IV', processes: ['diablo iv.exe'] },
  { id: 'poe', name: 'Path of Exile', processes: ['pathofexile.exe', 'pathofexilesteam.exe', 'pathofexile_x64.exe', 'pathofexile_x64steam.exe'] },
  { id: 'warframe', name: 'Warframe', processes: ['warframe.x64.exe'] },
  { id: 'destiny2', name: 'Destiny 2', processes: ['destiny2.exe'] },
  { id: 'helldivers2', name: 'Helldivers 2', processes: ['helldivers2.exe'] },
  { id: 'genshin', name: 'Genshin Impact', processes: ['genshinimpact.exe'] },
  { id: 'war-thunder', name: 'War Thunder', processes: ['aces.exe'] },
  { id: 'wot', name: 'World of Tanks', processes: ['worldoftanks.exe'] },
  { id: 'minecraft', name: 'Minecraft', processes: ['minecraft.windows.exe'] },
  { id: 'roblox', name: 'Roblox', processes: ['robloxplayerbeta.exe', 'robloxplayer'] },
  { id: 'elden-ring', name: 'Elden Ring', processes: ['eldenring.exe'] },
  { id: 'bg3', name: "Baldur's Gate 3", processes: ['bg3.exe', 'bg3_dx11.exe'] },
  { id: 'cyberpunk', name: 'Cyberpunk 2077', processes: ['cyberpunk2077.exe'] },
  { id: 'witcher3', name: 'The Witcher 3', processes: ['witcher3.exe'] },
  { id: 'hogwarts', name: 'Hogwarts Legacy', processes: ['hogwartslegacy.exe'] },
  { id: 'palworld', name: 'Palworld', processes: ['palworld-win64-shipping.exe'] },
  { id: 'dbd', name: 'Dead by Daylight', processes: ['deadbydaylight-win64-shipping.exe'] },
  { id: 'phasmophobia', name: 'Phasmophobia', processes: ['phasmophobia.exe'] },
  { id: 'among-us', name: 'Among Us', processes: ['among us.exe'] },
  { id: 'terraria', name: 'Terraria', processes: ['terraria.exe'] },
  { id: 'stardew', name: 'Stardew Valley', processes: ['stardew valley.exe', 'stardew valley'] },
  { id: 'l4d2', name: 'Left 4 Dead 2', processes: ['left4dead2.exe'] },
  { id: 'gmod', name: "Garry's Mod", processes: ['gmod.exe'] },
  { id: 'sea-of-thieves', name: 'Sea of Thieves', processes: ['sotgame.exe'] },
  { id: 'halo-infinite', name: 'Halo Infinite', processes: ['haloinfinite.exe'] },
  { id: 'fall-guys', name: 'Fall Guys', processes: ['fallguys_client_game.exe'] },
  { id: 'ea-fc', name: 'EA Sports FC', processes: ['fc27.exe', 'fc26.exe', 'fc25.exe'] },
  { id: 'lost-ark', name: 'Lost Ark', processes: ['lostark.exe'] },
  { id: 'black-desert', name: 'Black Desert', processes: ['blackdesert64.exe'] },
  { id: 'albion', name: 'Albion Online', processes: ['albion-online.exe'] },
  { id: 'osu', name: 'osu!', processes: ['osu!.exe'] },
];
const ACTIVITY_POLL_MS = 10000;
const ACTIVITY_CONFIRM_POLLS = 2;

let activityTimer = null;
let activityState = null;
let activityCandidate = null;
let activityPositive = 0;
let activityNegative = 0;

function listLinuxProcessNames() {
  const names = new Set();
  let entries = [];
  try {
    entries = fs.readdirSync('/proc');
  } catch {
    return names;
  }
  for (const entry of entries) {
    if (!/^\d+$/.test(entry)) continue;
    let name = '';
    try {
      const cmdline = fs.readFileSync(`/proc/${entry}/cmdline`, 'utf8');
      name = cmdline.split('\0')[0] || '';
    } catch {}
    if (!name) {
      try {
        name = fs.readFileSync(`/proc/${entry}/comm`, 'utf8').trim();
      } catch {}
    }
    if (!name) continue;
    names.add(name.split(/[\\/]/).pop().toLowerCase());
  }
  return names;
}

function listProcessNames() {
  if (process.platform === 'linux') return Promise.resolve(listLinuxProcessNames());
  return new Promise((resolve) => {
    const isWin = process.platform === 'win32';
    const cmd = isWin ? 'tasklist /FO CSV /NH' : 'ps -axo comm=';
    exec(cmd, { windowsHide: true, maxBuffer: 8 * 1024 * 1024 }, (err, stdout) => {
      if (err || !stdout) {
        resolve(new Set());
        return;
      }
      const names = new Set();
      for (const rawLine of stdout.split('\n')) {
        const line = rawLine.trim();
        if (!line) continue;
        let name = line;
        if (isWin) {
          const match = line.match(/^"([^"]+)"/);
          if (!match) continue;
          name = match[1];
        }
        name = name.split(/[\\/]/).pop();
        names.add(name.toLowerCase());
      }
      resolve(names);
    });
  });
}

function detectGame(names) {
  for (const game of GAME_CATALOG) {
    if (game.processes.some((p) => names.has(p.toLowerCase()))) return game;
  }
  return null;
}

function emitActivity() {
  if (mainWindow && !mainWindow.isDestroyed()) {
    mainWindow.webContents.send('activity:changed', activityState);
  }
}

async function pollGameActivity() {
  const names = await listProcessNames();
  const game = detectGame(names);
  if (game) {
    if (activityCandidate && activityCandidate.id === game.id) {
      activityPositive += 1;
    } else {
      activityCandidate = game;
      activityPositive = 1;
    }
    activityNegative = 0;
    const differs = !activityState || activityState.gameId !== game.id;
    if (differs && activityPositive >= ACTIVITY_CONFIRM_POLLS) {
      activityState = { gameId: game.id, name: game.name, startedAt: new Date().toISOString() };
      emitActivity();
    }
    return;
  }
  activityCandidate = null;
  activityPositive = 0;
  activityNegative += 1;
  if (activityState && activityNegative >= ACTIVITY_CONFIRM_POLLS) {
    activityState = null;
    emitActivity();
  }
}

function startGameActivityWatcher() {
  if (activityTimer) return;
  void pollGameActivity();
  activityTimer = setInterval(() => {
    void pollGameActivity();
  }, ACTIVITY_POLL_MS);
}

ipcMain.handle('activity:get', () => activityState);

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
    : pathToFileURL(path.join(__dirname, '../build/index.html')).href;

  watchFirstPaint(mainWindow);
  mainWindow.loadURL(startURL);
  startGameActivityWatcher();

  mainWindow.webContents.setWindowOpenHandler(({ url }) => {
    if (url.startsWith('http://') || url.startsWith('https://')) {
      shell.openExternal(url);
    }
    return { action: 'deny' };
  });

  mainWindow.webContents.on('will-navigate', (event, url) => {
    if (url !== startURL && !url.startsWith('http://localhost:3000')) {
      event.preventDefault();
      if (url.startsWith('http://') || url.startsWith('https://')) {
        shell.openExternal(url);
      }
    }
  });

  mainWindow.once('ready-to-show', () => {
    closeSplash();
    mainWindow.show();
  });

  mainWindow.webContents.on('render-process-gone', (_event, details) => {
    if (details.reason !== 'clean-exit' && details.reason !== 'killed') {
      relaunchWithoutGpu('renderer gone (' + details.reason + ')');
    }
  });

  mainWindow.webContents.once('did-fail-load', (_event, code, description) => {
    console.error('[window] failed to load: ' + code + ' ' + description);
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

  mainWindow.on('enter-full-screen', () => {
    mainWindow?.webContents.send('window:fullscreen', true);
  });
  mainWindow.on('leave-full-screen', () => {
    mainWindow?.webContents.send('window:fullscreen', false);
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

const GPU_FALLBACK_FLAG = '--rift-no-gpu';
const gpuDisabled = process.argv.includes(GPU_FALLBACK_FLAG) || process.argv.includes('--disable-gpu');
if (gpuDisabled) {
  app.disableHardwareAcceleration();
}

let relaunchedWithoutGpu = false;
function relaunchWithoutGpu(reason) {
  if (gpuDisabled || relaunchedWithoutGpu) return;
  relaunchedWithoutGpu = true;
  console.warn('[gpu] ' + reason + '; relaunching without hardware acceleration');
  app.relaunch({ args: process.argv.slice(1).concat(GPU_FALLBACK_FLAG) });
  app.exit(0);
}

const FIRST_PAINT_TIMEOUT_MS = 45000;
const BLANK_CHECK_DELAY_MS = 1500;
const CAPTURE_ATTEMPTS = 3;

function looksBlank(image) {
  const size = image.getSize();
  if (!size.width || !size.height) return false;
  const bitmap = image.toBitmap();
  const stride = 4;
  const total = size.width * size.height;
  const samples = 64;
  let first = null;
  for (let i = 0; i < samples; i++) {
    const px = Math.floor((i / samples) * total) * stride;
    const key = bitmap[px] + ',' + bitmap[px + 1] + ',' + bitmap[px + 2];
    if (first === null) first = key;
    else if (key !== first) return false;
  }
  return true;
}

function watchFirstPaint(win) {
  if (gpuDisabled) return;
  let settled = false;
  const settle = () => {
    if (settled) return false;
    settled = true;
    clearTimeout(timeout);
    ipcMain.removeListener('renderer:rendered', onRendered);
    return true;
  };
  const timeout = setTimeout(() => {
    if (settle()) relaunchWithoutGpu('renderer never reported its first paint');
  }, FIRST_PAINT_TIMEOUT_MS);
  let readyToShow = false;
  win.once('ready-to-show', () => {
    readyToShow = true;
  });
  const onRendered = () => {
    if (!settle()) return;
    setTimeout(() => {
      if (win.isDestroyed()) return;
      if (!readyToShow) {
        relaunchWithoutGpu('the app mounted but the window never became ready to show');
        return;
      }
      if (!win.isVisible() || win.isMinimized()) return;
      let attempts = 0;
      const capture = () => {
        if (win.isDestroyed()) return;
        attempts += 1;
        win.webContents
          .capturePage()
          .then((image) => {
            if (!win.isDestroyed() && looksBlank(image)) relaunchWithoutGpu('window painted a single colour after the app mounted');
          })
          .catch((err) => {
            console.warn('[gpu] capturePage failed (attempt ' + attempts + '):', err);
            if (attempts < CAPTURE_ATTEMPTS) setTimeout(capture, BLANK_CHECK_DELAY_MS);
            else relaunchWithoutGpu('the window could not be captured after the app mounted');
          });
      };
      capture();
    }, BLANK_CHECK_DELAY_MS);
  };
  ipcMain.once('renderer:rendered', onRendered);
  win.once('closed', () => {
    settle();
  });
}

app.on('child-process-gone', (_event, details) => {
  if (details.type === 'GPU' && details.reason !== 'clean-exit') {
    relaunchWithoutGpu('GPU process gone (' + details.reason + ')');
  }
});

app.on('ready', () => {
  Menu.setApplicationMenu(null);
  if (process.defaultApp && process.argv.length >= 2) {
    app.setAsDefaultProtocolClient(DEEP_LINK_PROTOCOL, process.execPath, [path.resolve(process.argv[1])]);
  } else {
    app.setAsDefaultProtocolClient(DEEP_LINK_PROTOCOL);
  }

  session.defaultSession.webRequest.onHeadersReceived((details, callback) => {
    const headers = { ...details.responseHeaders };
    headers['Cross-Origin-Opener-Policy'] = ['same-origin'];
    headers['Cross-Origin-Embedder-Policy'] = ['credentialless'];
    callback({ responseHeaders: headers });
  });

  const ALLOWED_PERMISSIONS = new Set(['media', 'display-capture', 'fullscreen', 'notifications']);

  const isTrustedSource = (value) => {
    if (!value) return false;
    return isDev
      ? value.startsWith('http://localhost:3000')
      : value.startsWith('file://');
  };

  const contentsUrl = (wc) => {
    try {
      return wc && typeof wc.getURL === 'function' ? wc.getURL() : '';
    } catch (_e) {
      return '';
    }
  };

  session.defaultSession.setPermissionCheckHandler(
    (wc, permission, requestingOrigin) =>
      ALLOWED_PERMISSIONS.has(permission) &&
      (isTrustedSource(contentsUrl(wc)) || isTrustedSource(requestingOrigin)),
  );
  session.defaultSession.setPermissionRequestHandler((wc, permission, cb) => {
    cb(ALLOWED_PERMISSIONS.has(permission) && isTrustedSource(contentsUrl(wc)));
  });

  const pendingScreenShare = new Map();
  let screenShareSeq = 0;

  ipcMain.on('screen-share:picked', (_event, { requestId, sourceId }) => {
    const entry = pendingScreenShare.get(requestId);
    if (!entry) return;
    pendingScreenShare.delete(requestId);
    clearTimeout(entry.timer);
    const chosen = entry.sources.find((s) => s.id === sourceId);
    if (!chosen) {
      entry.callback({});
      return;
    }
    const stream = { video: chosen };
    if (process.platform === 'win32') {
      stream.audio = 'loopback';
    }
    entry.callback(stream);
  });

  session.defaultSession.setDisplayMediaRequestHandler(
    (request, callback) => {
      desktopCapturer
        .getSources({
          types: ['screen', 'window'],
          thumbnailSize: { width: 320, height: 180 },
        })
        .then((sources) => {
          if (!sources.length || !mainWindow || mainWindow.isDestroyed()) {
            callback({});
            return;
          }
          const requestId = ++screenShareSeq;
          const timer = setTimeout(() => {
            if (pendingScreenShare.has(requestId)) {
              pendingScreenShare.delete(requestId);
              callback({});
            }
          }, 60000);
          pendingScreenShare.set(requestId, { sources, callback, timer });
          mainWindow.webContents.send('screen-share:sources', {
            requestId,
            sources: sources.map((s) => ({
              id: s.id,
              name: s.name,
              kind: s.id.startsWith('screen:') ? 'screen' : 'window',
              thumbnail: s.thumbnail.toDataURL(),
            })),
          });
        })
        .catch((err) => {
          console.error('[screen-share] getSources failed:', err);
          callback({});
        });
    },
    { useSystemPicker: true },
  );
  console.log('[screen-share] display media request handler registered');

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

ipcMain.handle('window:is-fullscreen', () => mainWindow?.isFullScreen() ?? false);
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

let uiohookModule;
let uiohookRunning = false;
let uiohookListening = false;
let pttKeycode = null;
let pttDown = false;

function loadUiohook() {
  if (uiohookModule !== undefined) return uiohookModule;
  try {
    uiohookModule = require('uiohook-napi');
  } catch (e) {
    console.warn('[ptt] the global key hook is unavailable:', e.message);
    uiohookModule = null;
  }
  return uiohookModule;
}

function uiohookKeycode(code) {
  const mod = loadUiohook();
  if (!mod) return null;
  const aliases = {
    ControlLeft: 'Ctrl',
    ControlRight: 'CtrlRight',
    AltLeft: 'Alt',
    ShiftLeft: 'Shift',
    MetaLeft: 'Meta',
  };
  let name = aliases[code] || code;
  if (/^Key[A-Z]$/.test(code)) name = code.slice(3);
  else if (/^Digit[0-9]$/.test(code)) name = code.slice(5);
  const keycode = mod.UiohookKey[name];
  return typeof keycode === 'number' ? keycode : null;
}

function sendPttState(down, event) {
  if (pttDown === down) return;
  pttDown = down;
  if (mainWindow && !mainWindow.isDestroyed()) {
    mainWindow.webContents.send('ptt:state', {
      down,
      altKey: !!event?.altKey,
      ctrlKey: !!event?.ctrlKey,
      metaKey: !!event?.metaKey,
      shiftKey: !!event?.shiftKey,
    });
  }
}

function stopGlobalPtt() {
  pttKeycode = null;
  sendPttState(false);
  if (!uiohookRunning) return;
  try {
    loadUiohook().uIOhook.stop();
  } catch (e) {
    console.warn('[ptt] stopping the global key hook failed:', e.message);
  }
  uiohookRunning = false;
}

function startGlobalPtt(code) {
  const mod = loadUiohook();
  if (!mod) return false;
  const keycode = uiohookKeycode(code);
  if (keycode === null) return false;
  pttKeycode = keycode;
  if (uiohookRunning) return true;
  if (!uiohookListening) {
    mod.uIOhook.on('keydown', (event) => {
      if (event.keycode === pttKeycode) sendPttState(true, event);
    });
    mod.uIOhook.on('keyup', (event) => {
      if (event.keycode === pttKeycode) sendPttState(false, event);
    });
    uiohookListening = true;
  }
  try {
    mod.uIOhook.start();
  } catch (e) {
    console.warn('[ptt] starting the global key hook failed:', e.message);
    pttKeycode = null;
    return false;
  }
  uiohookRunning = true;
  return true;
}

ipcMain.handle('ptt:global', (_event, code) => {
  if (typeof code !== 'string' || code.length === 0) {
    stopGlobalPtt();
    return false;
  }
  return startGlobalPtt(code);
});

app.on('will-quit', () => {
  stopGlobalPtt();
});
