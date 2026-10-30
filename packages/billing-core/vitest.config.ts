import { defineConfig } from "vitest/config";

// Standalone config so vitest never picks up the Vidya app's root vite config.
export default defineConfig({
  test: {
    root: import.meta.dirname,
    include: ["test/**/*.test.ts"],
    environment: "node",
  },
});
