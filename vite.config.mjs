import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const { devPort } = require('./package.json');

export default defineConfig({
  plugins: [react()],
  base: './',
  server: {
    port: devPort,
    strictPort: true, // fail if the port is taken — never silently grab another app's port
    open: false,
  },
  build: {
    chunkSizeWarningLimit: 2000, // Electron app - code-splitting has no benefit; silences expected warning
  },
});
