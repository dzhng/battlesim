// Which build is running: the app's commit, fixed when the bundle was built,
// and a fingerprint of the simulation module as the browser actually loaded
// it, so a stale cache or an unrebuilt module shows on screen.
import wasmUrl from "@wasm/game_wasm_bg.wasm?url";

declare const __APP_COMMIT__: string;

/** The app's commit (`-dirty`: built from uncommitted changes). */
export const APP_COMMIT: string = typeof __APP_COMMIT__ === "string" ? __APP_COMMIT__ : "unknown";

let simHash: Promise<string> | null = null;

/** The first 8 hex digits of the SHA-256 of the simulation module served to
 *  this page (from the HTTP cache once the battle has loaded it). */
export function simFingerprint(): Promise<string> {
  return (simHash ??= fetch(wasmUrl)
    .then((r) => r.arrayBuffer())
    .then((bytes) => crypto.subtle.digest("SHA-256", bytes))
    .then((d) =>
      Array.from(new Uint8Array(d, 0, 4), (b) => b.toString(16).padStart(2, "0")).join(""),
    )
    .catch(() => "unknown"));
}
