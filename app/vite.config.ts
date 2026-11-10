import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

// NEAR's client libraries reference `global` and `process.env`; the browser has
// neither. `define` rewrites them at build time, which is cheaper and more
// reliable than catching every reference at runtime.
export default defineConfig({
  plugins: [react()],
  define: {
    global: 'globalThis',
  },
  optimizeDeps: {
    esbuildOptions: {
      define: { global: 'globalThis' },
    },
  },
});
