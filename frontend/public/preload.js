const { contextBridge, ipcRenderer } = require('electron');

const initialHref = window.location.href;
const appBase = initialHref.replace(/[^/]*$/, '');

contextBridge.exposeInMainWorld('electron', {
  minimize: () => ipcRenderer.send('window-minimize'),
  reportRendered: () => ipcRenderer.send('renderer:rendered'),
  maximize: () => ipcRenderer.send('window-maximize'),
  close: () => ipcRenderer.send('window-close'),

  platform: process.platform,
  appBase,

  secureGet: () => ipcRenderer.invoke('secure:get'),
  secureSet: (value) => ipcRenderer.invoke('secure:set', value),
  secureClear: () => ipcRenderer.invoke('secure:clear'),

  onScreenShareSources: (callback) => {
    const handler = (_event, data) => callback(data);
    ipcRenderer.on('screen-share:sources', handler);
    return () => ipcRenderer.removeListener('screen-share:sources', handler);
  },
  pickScreenShareSource: (requestId, sourceId) =>
    ipcRenderer.send('screen-share:picked', { requestId, sourceId }),

  getGameActivity: () => ipcRenderer.invoke('activity:get'),
  onGameActivity: (callback) => {
    const handler = (_event, activity) => callback(activity ?? null);
    ipcRenderer.on('activity:changed', handler);
    return () => ipcRenderer.removeListener('activity:changed', handler);
  },

  onDeepLink: (callback) => {
    const handler = (_event, url) => {
      if (typeof url !== 'string') return;
      ipcRenderer.send('deep-link:consumed', url);
      callback(url);
    };
    ipcRenderer.on('deep-link', handler);
    return () => ipcRenderer.removeListener('deep-link', handler);
  },
  getPendingDeepLink: () => ipcRenderer.invoke('deep-link:pending'),

  setGlobalPushToTalk: (code) => ipcRenderer.invoke('ptt:global', code),
  onPushToTalk: (callback) => {
    const handler = (_event, state) =>
      callback({
        down: !!state?.down,
        altKey: !!state?.altKey,
        ctrlKey: !!state?.ctrlKey,
        metaKey: !!state?.metaKey,
        shiftKey: !!state?.shiftKey,
      });
    ipcRenderer.on('ptt:state', handler);
    return () => ipcRenderer.removeListener('ptt:state', handler);
  },

  isFullscreen: () => ipcRenderer.invoke('window:is-fullscreen'),
  onFullscreenChange: (callback) => {
    const handler = (_event, value) => callback(!!value);
    ipcRenderer.on('window:fullscreen', handler);
    return () => ipcRenderer.removeListener('window:fullscreen', handler);
  },
});
