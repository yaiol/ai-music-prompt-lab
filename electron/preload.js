const { contextBridge, webUtils } = require('electron');
// ⚠ CLAUDE: getFilePath is the only IPC bridge - needed for drag-and-drop file opening.
// webUtils.getPathForFile() is the official Electron 32+ replacement for file.path.
// See root CLAUDE.md "Electron IPC / Preload".
contextBridge.exposeInMainWorld('electronAPI', {
  getFilePath: (file) => webUtils.getPathForFile(file),
});
