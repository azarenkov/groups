const { contextBridge, ipcRenderer } = require('electron');

const initialHref = window.location.href;
const appBase = initialHref.replace(/[^/]*$/, '');

contextBridge.exposeInMainWorld('electron', {
  minimize: () => ipcRenderer.send('window-minimize'),
  maximize: () => ipcRenderer.send('window-maximize'),
  close: () => ipcRenderer.send('window-close'),

  platform: process.platform,
  appBase,

  secureGet: () => ipcRenderer.invoke('secure:get'),
  secureSet: (value) => ipcRenderer.invoke('secure:set', value),
  secureClear: () => ipcRenderer.invoke('secure:clear'),

  googleOAuth: () => ipcRenderer.invoke('oauth:google'),
});
