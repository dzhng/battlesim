import menuCapture from "@fixtures/captures/menu.json";
import waterCapture from "@fixtures/captures/water.json";

/** One scene of a reel: the capture file's scene, as the graphics test films it. */
export interface ReelScene {
  readonly map: string;
  readonly encounter: string;
  readonly seed: number;
  readonly warm_s: number;
  readonly reel: { readonly shots: readonly { readonly seconds: number }[] };
}

/** A reel the graphics test films, with a stable identity. */
export interface ReelWorkload {
  readonly id: string;
  readonly version: number;
  readonly scenes: readonly ReelScene[];
}

const workload = (id: string, capture: { scenes: readonly ReelScene[] }): ReelWorkload => ({
  id,
  version: 1,
  scenes: capture.scenes.map((scene) => ({
    map: scene.map,
    encounter: scene.encounter,
    seed: scene.seed,
    warm_s: scene.warm_s,
    reel: scene.reel,
  })),
});

/** The reels the graphics test films, by their `?capture=` name: the main
 *  menu's (the default, and the one the graphics tier is measured on) and
 *  the water scene's. The capture files own the content; this module only
 *  gives each a workload key. */
export const REELS = {
  menu: workload("menu-reel", menuCapture),
  water: workload("water", waterCapture),
} as const satisfies Record<string, ReelWorkload>;

export const MENU_REEL_WORKLOAD = REELS.menu;

/** The workload a run of `whole` that filmed `maps`, in that order,
 *  measured: the whole reel without them. A map the reel does not film is
 *  refused. */
export function reelWorkload(
  whole: ReelWorkload = MENU_REEL_WORKLOAD,
  maps?: readonly string[],
): ReelWorkload {
  if (!maps) return whole;
  return {
    ...whole,
    scenes: maps.map((map) => {
      const scene = whole.scenes.find((s) => s.map === map);
      if (!scene) throw new Error(`the ${whole.id} reel films no ${map}`);
      return scene;
    }),
  };
}

/** Stable hash of a reel workload's identity (the complete menu reel by default). */
export function reelFingerprint(workload: ReelWorkload = MENU_REEL_WORKLOAD): string {
  const text = JSON.stringify(workload);
  let hash = 0x811c9dc5;
  for (let i = 0; i < text.length; i++) {
    hash ^= text.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash.toString(16).padStart(8, "0");
}
