import { contextBridge, ipcRenderer, Notification } from 'electron';

contextBridge.exposeInMainWorld('electronAPI', {
    getConfig: () => ipcRenderer.invoke('get-config'),
    toggleAlwaysOnTop: () => ipcRenderer.send('toggle-always-on-top'),
    setWindowSize: (width, height, compact) =>
        ipcRenderer.send('set-window-size', { width, height, compact }),
    onConfigLoaded: (callback) => {
        ipcRenderer.invoke('get-config').then((config) => {
            window.__DASHBOARD_CONFIG__ = config;
            callback(config);
        });
    },
    showNotification: (title, body) => {
        if (Notification.isSupported()) {
            new Notification({ title, body }).show();
        }
    },
});
