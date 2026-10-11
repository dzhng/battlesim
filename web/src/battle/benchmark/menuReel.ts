import backdrop from "@fixtures/menu-backdrop.json";

/** Stable identity for the menu scenes used by the graphics test. The fixture
 * remains the content owner; this module only gives candidates a workload key. */
export const MENU_REEL_WORKLOAD = {
  id: "menu-reel",
  version: 1,
  scenes: backdrop.scenes.map((scene) => ({
    map: scene.map,
    encounter: scene.encounter,
    seed: scene.seed,
    warm_s: scene.warm_s,
    reel: scene.reel,
  })),
} as const;

/** Stable hash of the complete menu-reel workload identity. */
export function menuReelFingerprint(): string {
  const text = JSON.stringify(MENU_REEL_WORKLOAD);
  let hash = 0x811c9dc5;
  for (let i = 0; i < text.length; i++) {
    hash ^= text.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash.toString(16).padStart(8, "0");
}
