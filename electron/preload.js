const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("electron", {
  openMediaFile: () => ipcRenderer.invoke("open-media-file"),
});
