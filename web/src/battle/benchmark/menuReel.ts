import menuCapture from "@fixtures/captures/menu.json";

/** Stable identity for the menu scenes used by the graphics test. The fixture
 * remains the content owner; this module only gives candidates a workload key. */
export const MENU_REEL_WORKLOAD = {
  id: "menu-reel",
  version: 1,
  scenes: menuCapture.scenes.map((scene) => ({
    map: scene.map,
    encounter: scene.encounter,
    seed: scene.seed,
    warm_s: scene.warm_s,
    reel: scene.reel,
  })),
} as const;

export type MenuReelWorkload = Omit<typeof MENU_REEL_WORKLOAD, "scenes"> & {
  scenes: readonly (typeof MENU_REEL_WORKLOAD.scenes)[number][];
};

/** The workload a run that filmed `maps`, in that order, measured: the whole
 *  menu reel without them. A map the reel does not film is refused. */
export function menuReelWorkload(maps?: readonly string[]): MenuReelWorkload {
  if (!maps) return MENU_REEL_WORKLOAD;
  return {
    ...MENU_REEL_WORKLOAD,
    scenes: maps.map((map) => {
      const scene = MENU_REEL_WORKLOAD.scenes.find((s) => s.map === map);
      if (!scene) throw new Error(`the menu reel films no ${map}`);
      return scene;
    }),
  };
}

/** Stable hash of a menu-reel workload's identity (the complete reel by default). */
export function menuReelFingerprint(workload: MenuReelWorkload = MENU_REEL_WORKLOAD): string {
  const text = JSON.stringify(workload);
  let hash = 0x811c9dc5;
  for (let i = 0; i < text.length; i++) {
    hash ^= text.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash.toString(16).padStart(8, "0");
}
