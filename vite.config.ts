import { resolve } from "node:path";
import { defineConfig } from "vite";

// Tauri 固定端口 1420，见 src-tauri/tauri.conf.json devUrl
export default defineConfig({
  clearScreen: false,
  server: { port: 1420, strictPort: true, watch: { ignored: ["**/src-tauri/**"] } },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    // macOS 10.15 的 WKWebView 是 Safari 13
    target: process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari13",
    minify: !process.env.TAURI_ENV_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    // 两个页面：便签窗口 index.html、列表面板 list.html
    rollupOptions: { input: { main: resolve(__dirname, "index.html"), list: resolve(__dirname, "list.html") } },
  },
});
