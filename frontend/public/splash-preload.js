const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('riftSplash', {
  onStatus: (callback) => ipcRenderer.on('updater:status', (_, text) => callback(String(text))),
  onProgress: (callback) => ipcRenderer.on('updater:progress', (_, percent) => callback(Number(percent))),
});
