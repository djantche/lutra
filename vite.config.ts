import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],

  // Vite serves the interface while developing: `pnpm dev`, then
  // `LUTRA_DEV_URL=http://localhost:1420 pnpm exec electron .` in another terminal.
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
