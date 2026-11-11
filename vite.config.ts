import react from '@vitejs/plugin-react';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
  test: {
    environment: 'jsdom',
    setupFiles: './src/test/setup.ts',
    restoreMocks: true,
    // The full suite loads many isolated jsdom environments in parallel.
    // Generous per-test headroom prevents environment startup from being
    // reported as a functional timeout on slower machines.
    testTimeout: 15_000,
    hookTimeout: 15_000,
  },
});
