// Minimal vitest config for tests that don't need the full vite/wasm setup.
// Used for CLI tool tests like hdr encoding that only use Node.js built-ins.
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "node",
    include: ["tests/hdrEncodeE2e.test.ts", "tests/hdrEncode.test.ts"],
    globals: false,
    testTimeout: 60_000,
  },
});
