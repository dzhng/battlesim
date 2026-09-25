import { defineConfig, mergeConfig } from "vitest/config";
import viteConfig from "./vite.config";

// DOM-free suites select Vitest's node environment with a per-file directive.
export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      environment: "jsdom",
      include: ["tests/**/*.test.{ts,tsx}", "scene.test.mjs"],
      globals: false,
    },
  }),
);
