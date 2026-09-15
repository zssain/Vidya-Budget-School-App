import { defineConfig } from 'vitest/config';

// Tauri 2 + Vite configuration.
// Reference: Tauri v2 "Vite" frontend guide (matches @tauri-apps/cli 2.11.x).
// `TAURI_DEV_HOST` is set by the Tauri CLI for mobile dev so the dev server
// binds to a reachable host and HMR points back at it.
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  // The frontend lives in src/; index.html is src/index.html.
  root: 'src',

  // Vite options tailored for Tauri development.
  // Prevent Vite from clearing the screen so Rust/Tauri errors stay visible.
  clearScreen: false,

  build: {
    // Emit the built site to <project>/dist (relative to root = src).
    outDir: '../dist',
    emptyOutDir: true,
  },

  server: {
    // Tauri expects a fixed port; fail rather than silently pick another.
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // Rust files are watched by the Tauri CLI, not Vite.
      ignored: ['**/src-tauri/**'],
    },
  },

  test: {
    // Run tests from the project root so both src/ and scripts/ globs resolve,
    // independent of Vite's `root: 'src'` above.
    root: import.meta.dirname,
    environment: 'jsdom',
    include: ['src/**/*.test.js', 'scripts/**/*.test.js'],
  },
});
