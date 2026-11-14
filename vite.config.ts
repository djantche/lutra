import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],

  // Vite serves the interface while developing: `pnpm dev:electron` runs it inside Electron
  // (`LUTRA_DEV_URL` makes the shell load this server), `pnpm dev` alone is for a browser.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // The backend's build output is huge and irrelevant to the interface.
      ignored: ["**/backend/**", "**/release/**"],
    },
  },
});
