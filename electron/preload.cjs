// Exposes the few things the interface may ask of the shell, as `window.lutra`.
// Nothing else of Node or Electron reaches the page.
const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("lutra", {
  /** Runs a backend command. Rejects with the backend's message, as a plain string. */
  async invoke(command, args) {
    const reply = await ipcRenderer.invoke("rpc", command, args ?? {});
    if (reply.err !== undefined) throw reply.err;
    return reply.ok;
  },

  /** Listens to a backend event. Returns the function that stops listening. */
  on(event, callback) {
    const channel = `ev:${event}`;
    const listener = (_event, payload) => callback(payload);
    ipcRenderer.on(channel, listener);
    return () => ipcRenderer.removeListener(channel, listener);
  },

  /** Window controls: `minimize`, `toggleMaximize`, `close`, `setFullscreen` (with a boolean), `toTray`, `quit`. */
  window: (operation, value) => ipcRenderer.invoke("window", operation, value),

  /** Native file or folder picker. Resolves to the chosen path, or null. */
  pick: (options) => ipcRenderer.invoke("pick", options),

  version: () => ipcRenderer.invoke("version"),
});
