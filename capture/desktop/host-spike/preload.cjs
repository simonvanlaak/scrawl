const { contextBridge, ipcRenderer } = require('electron');

// No generic channel, filesystem, shell, navigation or recording capability.
contextBridge.exposeInMainWorld('hostSpike', {
  snapshot: () => ipcRenderer.invoke('spike:snapshot'),
  command: input => ipcRenderer.invoke('spike:command', input),
});
