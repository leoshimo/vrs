import { configurationServer } from './server/configuration';
import tailwindcss from '@tailwindcss/postcss';
import vinext from 'vinext';
import { defineConfig } from 'vite';
export default defineConfig({
  resolve: { dedupe: ['react', 'react-dom'] },
  css: { postcss: { plugins: [tailwindcss()] } },
  plugins: [configurationServer(), vinext()],
  server: { host: '127.0.0.1', port: 5173, strictPort: true },
});
