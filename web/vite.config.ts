import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { VitePWA } from "vite-plugin-pwa";

// Static offline PWA (REQUIREMENTS §5.8, goal §7). Relative base so the app
// opens from any subdirectory or plain file hosting.
export default defineConfig({
  base: "./",
  // Allow reading repository files (the example world worlds/*.toml lives above web/).
  server: { fs: { allow: [".."] } },
  plugins: [
    svelte(),
    VitePWA({
      registerType: "autoUpdate",
      // Put .wasm into the precache (needed offline); workbox skips it by default.
      workbox: { globPatterns: ["**/*.{js,css,html,wasm}"] },
      manifest: {
        name: "Dungeonomics — living economy",
        short_name: "Dungeonomics",
        description: "A living regional economy for tabletop RPGs",
        theme_color: "#1b1b1f",
        background_color: "#1b1b1f",
        display: "standalone",
        icons: [],
      },
    }),
  ],
});
