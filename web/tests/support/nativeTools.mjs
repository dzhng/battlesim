// Vitest global setup: the suites that ask the mechanics server or the map
// workbench run the native tools, so build them first, as the scene runner
// does. Cargo skips a fresh one; a stale one would read today's catalog with
// yesterday's schema and fail far from the cause.
import { ensureNativeTools } from "../../scene.mjs";

export default async function setup() {
  await ensureNativeTools();
}
