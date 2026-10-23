// Fixed port and relaxed watch so `tauri dev` can find the dev server without rebuilding Rust.
import { defineConfig } from "vite";

export default defineConfig({
  clearScreen: false,
  // dist/ holds the published executable, so the frontend build goes to its own folder
  build: { outDir: "web-dist" },
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
