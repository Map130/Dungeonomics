import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { VitePWA } from "vite-plugin-pwa";

// Статический офлайн-PWA (REQUIREMENTS §5.8, цель §7). База относительная, чтобы
// приложение открывалось из любого подкаталога/файловой раздачи.
export default defineConfig({
  base: "./",
  // Разрешаем читать файлы репозитория (пример мира worlds/*.toml лежит выше web/).
  server: { fs: { allow: [".."] } },
  plugins: [
    svelte(),
    VitePWA({
      registerType: "autoUpdate",
      // .wasm попадает в precache (нужен офлайн); по умолчанию workbox его не берёт.
      workbox: { globPatterns: ["**/*.{js,css,html,wasm}"] },
      manifest: {
        name: "D&D Eco — живая экономика",
        short_name: "D&D Eco",
        description: "Живая региональная экономика для настольных RPG",
        theme_color: "#1b1b1f",
        background_color: "#1b1b1f",
        display: "standalone",
        icons: [],
      },
    }),
  ],
});
