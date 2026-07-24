import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  server: {
    port: 5173,
    proxy: {
      // Forward API calls to the kowalski HTTP server. Keep in sync with
      // `kowalski_core::config::DEFAULT_API_BIND` (single source of truth on the Rust side).
      "/api": {
        target: "http://127.0.0.1:3456",
        changeOrigin: true,
      },
    },
  },
});
