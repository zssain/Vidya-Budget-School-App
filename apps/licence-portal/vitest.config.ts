import path from "node:path";
import { defineConfig } from "vitest/config";

export default defineConfig({
  resolve: {
    alias: {
      "@": path.join(import.meta.dirname, "src"),
      // Next resolves `server-only` with the react-server condition; tests run on the server too.
      "server-only": path.join(import.meta.dirname, "test/empty.ts"),
    },
  },
  test: {
    root: import.meta.dirname,
    include: ["test/**/*.test.ts"],
    environment: "node",
    testTimeout: 30000,
    hookTimeout: 60000,
  },
});
