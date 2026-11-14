// `pnpm dev:electron`: the interface from the Vite dev server (hot reload) inside the real Electron
// shell, with a debug build of the backend. Stops Vite when Electron closes.

import { spawn, spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");
const bin = (name) => path.join(root, "node_modules", ".bin", name);
const PORT = 1420; // vite.config.ts: strictPort, so it is always this one
const URL = `http://localhost:${PORT}`;

// The shell prefers a release build when one exists: point it at the one just built instead.
const backend = path.join(root, "backend", "target", "debug", "lutra-backend");
const build = spawnSync("cargo", ["build", "--manifest-path", path.join(root, "backend", "Cargo.toml")], {
  stdio: "inherit",
});
if (build.status !== 0) process.exit(build.status ?? 1);

const vite = spawn(bin("vite"), [], { cwd: root, stdio: "inherit" });
let stopping = false;
const stop = (code = 0) => {
  if (stopping) return;
  stopping = true;
  vite.kill();
  process.exit(code);
};
vite.on("exit", (code) => stop(code ?? 1));
for (const signal of ["SIGINT", "SIGTERM"]) process.on(signal, () => stop());

// Electron would load an empty page if it started before the server answers.
for (let attempt = 0; ; attempt++) {
  const ready = await fetch(URL).then(
    () => true,
    () => false,
  );
  if (ready) break;
  if (attempt > 100) {
    console.error(`The Vite server did not answer on ${URL}`);
    stop(1);
  }
  await new Promise((resolve) => setTimeout(resolve, 100));
}

const electron = spawn(bin("electron"), ["."], {
  cwd: root,
  stdio: "inherit",
  env: { ...process.env, LUTRA_DEV_URL: URL, LUTRA_BACKEND: backend },
});
const started = Date.now();
electron.on("exit", (code) => {
  // Lutra allows one instance: a second one hands over to the first and leaves at once.
  if (Date.now() - started < 5000) console.error("Electron closed right away: is another Lutra already running? Close it first.");
  stop(code ?? 0);
});
