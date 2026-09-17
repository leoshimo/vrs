import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  base: "./",
  clearScreen: false,
  build: { target: "safari17" },
  server: { host: "127.0.0.1", port: 5175, strictPort: true },
});
