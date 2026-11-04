// The cross-stack purchase rehearsal (scripts/rehearse-purchase.sh). Separate from the
// normal suite: it needs the real Rust signer binary + a rehearsal directory.
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    root: import.meta.dirname,
    include: ["rehearsal/**/*.rehearsal.test.ts"],
    environment: "node",
    fileParallelism: false,
    sequence: { concurrent: false },
    testTimeout: 120000,
    hookTimeout: 60000,
  },
});
