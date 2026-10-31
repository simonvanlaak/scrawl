const { app, BrowserWindow, ipcMain, session } = require('electron');
const path = require('node:path');
const { pathToFileURL } = require('node:url');

// A separate ordinary window. Never registers shortcuts, overlays or URL handlers.
// Require a disposable profile so running this spike cannot use Apparent's store.
const profile = process.env.HOST_SPIKE_PROFILE;
if (!profile || !path.isAbsolute(profile)) {
  console.error('Set HOST_SPIKE_PROFILE to an absolute disposable directory.');
  app.exit(2);
} else {
  app.setPath('userData', profile);
  app.whenReady().then(async () => {
    const { createHost } = await import('./host.mjs');
    const host = createHost();
    const entry = pathToFileURL(path.join(__dirname, 'dist/index.html')).href;
    session.defaultSession.setPermissionRequestHandler((_contents, _permission, callback) => callback(false));
    session.defaultSession.setPermissionCheckHandler(() => false);
    session.defaultSession.webRequest.onBeforeRequest((details, callback) => {
      const allowed = ['index.html', 'app.mjs', 'style.css'].some(name =>
        details.url === pathToFileURL(path.join(__dirname, 'dist', name)).href);
      callback({ cancel: !allowed });
    });
    const win = new BrowserWindow({
      title: 'Apparent host spike — diagnostic only', width: 880, height: 680,
      show: false, backgroundColor: '#f5f3ef',
      webPreferences: {
        preload: path.join(__dirname, 'preload.cjs'),
        contextIsolation: true, sandbox: true, nodeIntegration: false,
        webSecurity: true, devTools: false,
      },
    });
    const trusted = event => event.sender === win.webContents &&
      event.senderFrame === win.webContents.mainFrame && event.senderFrame.url === entry;
    ipcMain.handle('spike:snapshot', event => {
      if (!trusted(event)) throw new Error('untrusted_sender');
      return host.snapshot();
    });
    ipcMain.handle('spike:command', (event, input) => {
      if (!trusted(event) || JSON.stringify(input).length > 2048) throw new Error('invalid_sender_or_size');
      return host.command(input);
    });
    win.webContents.setWindowOpenHandler(() => ({ action: 'deny' }));
    win.webContents.on('will-navigate', event => event.preventDefault());
    win.once('ready-to-show', () => win.show());
    win.once('closed', () => {
      ipcMain.removeHandler('spike:snapshot');
      ipcMain.removeHandler('spike:command');
      app.quit();
    });
    await win.loadFile(path.join(__dirname, 'dist/index.html'));
  }).catch(error => { console.error(error.message); app.exit(1); });
  app.on('window-all-closed', () => app.quit());
}
