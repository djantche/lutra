// Lutra's shell: the windows, the tray icon and the dialogs. Everything that touches the system
// is done by the Rust backend (`backend/`), a child process we talk to in JSON lines.
const { app, BrowserWindow, Menu, Tray, dialog, ipcMain, nativeImage, net, protocol } = require("electron");
const { spawn } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");
const readline = require("node:readline");
const { pathToFileURL } = require("node:url");

const DEV_URL = process.env.LUTRA_DEV_URL; // the Vite server, when developing the interface
const ICON = path.join(__dirname, "icon.png");
const DIST = path.join(__dirname, "..", "dist");
const APP_URL = "lutra://app/index.html";

// The interface is served from `dist` under its own scheme, so absolute asset paths keep working.
protocol.registerSchemesAsPrivileged([
  { scheme: "lutra", privileges: { standard: true, secure: true, supportFetchAPI: true, corsEnabled: true } },
]);

app.commandLine.appendSwitch("ozone-platform-hint", "auto");
// Interface sounds play without waiting for a click first.
app.commandLine.appendSwitch("autoplay-policy", "no-user-gesture-required");
Menu.setApplicationMenu(null);

/** @type {import('electron').BrowserWindow | null} */
let mainWindow = null;
/** @type {import('electron').BrowserWindow | null} */
let overlayWindow = null;
let tray = null;
let quitting = false;
let hiddenForGame = false;
let trayLabels = { open: "Lutra", bigPicture: "Big Picture", quit: "Quit" };
let trayInfo = { available: false, closeToTray: false, startInTray: false };
let trayKnown = null; // resolves once the backend has told us about the notification area
let trayKnownResolve = () => {};

// ---------------------------------------------------------------- backend

let backend = null;
let nextId = 1;
const pending = new Map();

function backendPath() {
  if (process.env.LUTRA_BACKEND) return process.env.LUTRA_BACKEND;
  if (app.isPackaged) return path.join(process.resourcesPath, "lutra-backend");
  for (const profile of ["release", "debug"]) {
    const candidate = path.join(__dirname, "..", "backend", "target", profile, "lutra-backend");
    if (fs.existsSync(candidate)) return candidate;
  }
  return "lutra-backend";
}

function startBackend() {
  // The backend reads the options of the command line too (`--big-picture`).
  const flags = process.argv.slice(1).filter((arg) => arg === "--big-picture");
  backend = spawn(backendPath(), flags, { stdio: ["pipe", "pipe", "inherit"] });
  backend.on("error", (error) => fail(`The backend could not start: ${error.message}`));
  backend.on("exit", (code) => {
    if (!quitting) fail(`The backend stopped unexpectedly (code ${code}).`);
  });
  readline.createInterface({ input: backend.stdout }).on("line", onBackendLine);
}

function fail(message) {
  if (quitting) return;
  quitting = true;
  dialog.showErrorBox("Lutra", message);
  app.quit();
}

/** Runs a backend command. Resolves to `{ ok }` or `{ err }`. */
function call(command, args = {}) {
  return new Promise((resolve) => {
    const id = nextId++;
    pending.set(id, resolve);
    backend.stdin.write(`${JSON.stringify({ id, cmd: command, args })}\n`);
  });
}

function onBackendLine(line) {
  let message;
  try {
    message = JSON.parse(line);
  } catch {
    return;
  }
  if (message.event !== undefined) return onEvent(message.event, message.payload);
  const resolve = pending.get(message.id);
  if (!resolve) return;
  pending.delete(message.id);
  if (message.err !== undefined) return resolve({ err: message.err });
  const ok = message.ok;
  // Binary results (artwork) come as base64.
  resolve({ ok: ok && typeof ok === "object" && "$bytes" in ok ? Buffer.from(ok.$bytes, "base64") : ok });
}

// ---------------------------------------------------------------- events

function onEvent(event, payload) {
  switch (event) {
    case "host:tray":
      trayInfo = payload;
      updateTray();
      trayKnownResolve();
      return;
    case "host:game":
      return onGame(payload);
    case "host:overlay":
      return payload.open ? openOverlay(payload) : closeOverlay();
    case "host:show":
      return showMain();
    case "ui-action":
      return routeAction(payload);
    default:
      for (const window of BrowserWindow.getAllWindows()) window.webContents.send(`ev:${event}`, payload);
  }
}

/** Controller actions go to the overlay when it is open, else to Lutra when it is in use. */
function routeAction(action) {
  if (overlayWindow) return overlayWindow.webContents.send("ev:ui-action", action);
  // Powering the controller off acts whatever window is in use.
  if (mainWindow && (action === "powerOff" || mainWindow.isFocused())) {
    mainWindow.webContents.send("ev:ui-action", action);
  }
}

/** A game started or ended: the window steps aside while games run, and comes back after. */
function onGame({ anyRunning, hide }) {
  if (anyRunning) {
    if (hide && mainWindow && mainWindow.isVisible()) {
      mainWindow.hide();
      hiddenForGame = true;
    }
  } else if (hiddenForGame) {
    hiddenForGame = false;
    showMain();
  }
}

// ---------------------------------------------------------------- windows

function webPreferences() {
  return { preload: path.join(__dirname, "preload.cjs"), contextIsolation: true, sandbox: true, spellcheck: false };
}

function pageUrl(query = "") {
  return DEV_URL ? `${DEV_URL}/${query}` : `${APP_URL}${query}`;
}

function createMainWindow() {
  mainWindow = new BrowserWindow({
    width: 1280,
    height: 800,
    minWidth: 960,
    minHeight: 600,
    show: false,
    frame: false,
    title: "Lutra",
    icon: ICON,
    backgroundColor: "#0b0c12",
    webPreferences: webPreferences(),
  });
  mainWindow.on("page-title-updated", (event) => event.preventDefault());
  mainWindow.on("close", (event) => {
    if (!quitting && trayInfo.closeToTray) {
      event.preventDefault();
      mainWindow.hide();
    }
  });
  mainWindow.on("closed", () => {
    mainWindow = null;
    if (overlayWindow) overlayWindow.close();
  });
  mainWindow.loadURL(pageUrl());
}

function showMain() {
  if (!mainWindow) return;
  if (mainWindow.isMinimized()) mainWindow.restore();
  mainWindow.show();
  mainWindow.focus();
}

function openOverlay({ title, fullscreen }) {
  if (overlayWindow) return;
  overlayWindow = new BrowserWindow({
    width: 1280,
    height: 720,
    title,
    frame: false,
    transparent: true,
    fullscreen,
    alwaysOnTop: true,
    skipTaskbar: true,
    show: false,
    backgroundColor: "#00000000",
    webPreferences: webPreferences(),
  });
  // Hyprland finds the window by its title: the page must not change it.
  overlayWindow.on("page-title-updated", (event) => event.preventDefault());
  overlayWindow.once("ready-to-show", () => {
    overlayWindow.show();
    overlayWindow.focus();
    call("overlay_shown", { pid: process.pid });
  });
  overlayWindow.on("closed", () => {
    overlayWindow = null;
    call("overlay_closed");
  });
  overlayWindow.loadURL(pageUrl("?overlay=1"));
}

function closeOverlay() {
  if (overlayWindow) overlayWindow.close();
}

// ---------------------------------------------------------------- tray

function trayMenu() {
  return Menu.buildFromTemplate([
    { label: trayLabels.open, click: showMain },
    {
      label: trayLabels.bigPicture,
      click: () => {
        showMain();
        if (mainWindow) mainWindow.webContents.send("ev:tray-big-picture", null);
      },
    },
    { type: "separator" },
    { label: trayLabels.quit, click: () => app.quit() },
  ]);
}

function updateTray() {
  if (!trayInfo.available) {
    if (tray) tray.destroy();
    tray = null;
    return;
  }
  if (!tray) {
    tray = new Tray(nativeImage.createFromPath(ICON).resize({ width: 32, height: 32 }));
    tray.setToolTip("Lutra");
    tray.on("click", showMain);
  }
  tray.setContextMenu(trayMenu());
}

// ---------------------------------------------------------------- requests from the interface

ipcMain.handle("rpc", async (_event, command, args) => {
  // The tray icon is ours, not the backend's.
  if (command === "set_tray_labels") {
    trayLabels = { open: args.labels.open, bigPicture: args.labels.bigPicture, quit: args.labels.quit };
    if (tray) tray.setContextMenu(trayMenu());
    return { ok: null };
  }
  return call(command, args);
});

ipcMain.handle("window", (event, operation, value) => {
  const window = BrowserWindow.fromWebContents(event.sender);
  if (!window) return;
  switch (operation) {
    case "minimize":
      return window.minimize();
    case "toggleMaximize":
      return window.isMaximized() ? window.unmaximize() : window.maximize();
    case "close":
      return window.close();
    case "setFullscreen":
      return window.setFullScreen(Boolean(value));
  }
});

ipcMain.handle("pick", async (event, options = {}) => {
  const window = BrowserWindow.fromWebContents(event.sender);
  const result = await dialog.showOpenDialog(window ?? undefined, {
    properties: [options.directory ? "openDirectory" : "openFile"],
    filters: options.filters,
  });
  return result.canceled ? null : (result.filePaths[0] ?? null);
});

ipcMain.handle("version", () => app.getVersion());

// ---------------------------------------------------------------- life cycle

if (!app.requestSingleInstanceLock()) {
  // Another Lutra is running: it comes to the front, and this one is not needed.
  app.quit();
} else {
  app.on("second-instance", showMain);

  app.whenReady().then(async () => {
    protocol.handle("lutra", (request) => {
      const requested = decodeURIComponent(new URL(request.url).pathname);
      const file = path.normalize(path.join(DIST, requested === "/" ? "/index.html" : requested));
      if (!file.startsWith(DIST)) return new Response("Forbidden", { status: 403 });
      return net.fetch(pathToFileURL(file).toString());
    });

    trayKnown = new Promise((resolve) => {
      trayKnownResolve = resolve;
    });
    startBackend();
    createMainWindow();

    // The window shows once we know whether it may start hidden in the notification area.
    const ready = new Promise((resolve) => mainWindow.once("ready-to-show", resolve));
    await Promise.all([ready, Promise.race([trayKnown, new Promise((resolve) => setTimeout(resolve, 2000))])]);
    const startHidden = trayInfo.available && (trayInfo.startInTray || process.argv.includes("--minimized"));
    if (!startHidden && mainWindow) showMain();
  });

  app.on("before-quit", () => {
    quitting = true;
    if (backend) {
      // The backend exits when its input closes; the kill is only a safety net.
      backend.stdin.end();
      setTimeout(() => backend.kill(), 1500).unref();
    }
  });

  app.on("window-all-closed", () => app.quit());
}
