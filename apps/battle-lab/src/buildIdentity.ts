// Which build is running and where its source lives: the app's commit, fixed
// when the bundle was built, and a fingerprint of the simulation module as the
// browser actually loaded it, so a stale cache or an unrebuilt module shows on
// screen.
import wasmUrl from "@wasm/game_wasm_bg.wasm?url";

import type { BuildIdentity } from "../../../packages/multiplayer/src/wire";

declare const __APP_COMMIT__: string;
declare const __BUILD_COMMIT__: string;

/** Where the game's source lives: it is open source and takes contributions. */
export const SOURCE_REPO = "https://github.com/dzhng/battlesim";

/** The app's commit (`-dirty`: built from uncommitted changes). */
export const APP_COMMIT: string = typeof __APP_COMMIT__ === "string" ? __APP_COMMIT__ : "unknown";

/** Full SHA-256 of the module the page actually loads. HTTP caches the bytes. */
export async function buildIdentity(): Promise<BuildIdentity> {
  const response = await fetch(wasmUrl);
  if (!response.ok) throw new Error("The simulation build could not be checked.");
  const digest = await crypto.subtle.digest("SHA-256", await response.arrayBuffer());
  return {
    commit: __BUILD_COMMIT__,
    sim: Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join(""),
  };
}

/** Keep the menu's existing eight-digit display and failed-load marker. */
export function simFingerprint(): Promise<string> {
  return buildIdentity()
    .then(({ sim }) => sim.slice(0, 8))
    .catch(() => "unknown");
}
