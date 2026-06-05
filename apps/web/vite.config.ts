import { fileURLToPath, URL } from "node:url";
import tailwindcss from "@tailwindcss/vite";
import vue from "@vitejs/plugin-vue";
import Icons from "unplugin-icons/vite";
import vueHsml from "vite-plugin-vue-hsml";
import { defineConfig } from "vitest/config";

export default defineConfig({
  // vueHsml must precede vue() so HSML templates compile to HTML before Vue parses them.
  plugins: [vueHsml(), vue(), tailwindcss(), Icons({ compiler: "vue3" })],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  server: {
    proxy: {
      // Dev: forward GraphQL (HTTP + WS) to the Rust API.
      "/graphql": {
        target: "http://localhost:8080",
        ws: true,
        changeOrigin: true,
      },
    },
  },
  test: {
    environment: "jsdom",
    globals: true,
    include: ["src/**/*.{test,spec}.ts"],
  },
});
