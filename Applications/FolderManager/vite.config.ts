// Fixed port and relaxed watch so `tauri dev` can find the dev server without rebuilding Rust.
import { defineConfig, type Plugin } from "vite";

// Linux users call a folder a directory, so a Linux build says so in every UI string; Tauri names the target platform.
// Only text with a space, or the bare word, is touched, and only the whole word, so class names, selectors and command names stay as written.
// Vite runs this under Node, whose process the browser-only tsconfig does not type.
const platform = (globalThis as { process?: { env: Record<string, string | undefined> } }).process?.env.TAURI_ENV_PLATFORM;

function linuxTerms(): Plugin {
  const word = /(?<![-_.\w])([Ff])older(s?)(?![-_\w])/g;
  const literal = /(["'`])((?:\\.|(?!\1)[^\\\n])*)\1/g;
  const swap = (text: string) => text.replace(word, (_, f: string, s: string) => (f === "F" ? "Director" : "director") + (s ? "ies" : "y"));
  return {
    name: "linux-terms",
    enforce: "pre",
    transform(code, id) {
      if (platform !== "linux" || !/\/src\/.*\.ts$/.test(id.replace(/\\/g, "/"))) return null;
      return code.replace(literal, (whole, q: string, body: string) => (/\s|^[Ff]olders?$/.test(body) ? q + swap(body) + q : whole));
    },
  };
}

export default defineConfig({
  plugins: [linuxTerms()],
  clearScreen: false,
  // dist/ holds the published executable, so the frontend build goes to its own folder
  build: { outDir: "web-dist" },
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
