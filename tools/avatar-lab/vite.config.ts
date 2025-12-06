import tailwindcss from '@tailwindcss/postcss';
import react from '@vitejs/plugin-react';
import { presetStore } from './dev/preset-store.ts';
import { defineConfig } from 'vite';
import { fileURLToPath } from 'node:url';
export default defineConfig({
  resolve: { alias: { '@': fileURLToPath(new URL('.', import.meta.url)) }, dedupe: ['react', 'react-dom'] },
  css: { postcss: { plugins: [tailwindcss()] } },
  plugins: [presetStore(), react()],
  server: { host: '127.0.0.1', port: 5173, strictPort: true },
});
