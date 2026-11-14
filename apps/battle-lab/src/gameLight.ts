import { light_records } from "@wasm/renderer_browser";
// The battle's light, from `fixtures/presentation.json`. Every
// lab route draws under it, as every lab reuses the game's numbers.
import { lightForPreset, presentationSettings } from "@web/presentationSettings";
import { type LightPresentation } from "@packages/battle-renderer/src/light/sceneLight";
import {
  DEFAULT_LIGHTING,
  lightWithMapSeed,
  parseLighting,
  resolveLighting,
} from "./lobby/lighting";

export const gameLight: LightPresentation = presentationSettings.light;

/** A saved replay's resolved look, applied when the URL has no `?preset=`. */
let matchLighting: string | null = null;

/** Record the resolved lighting a saved battle should play under. */
export function setMatchLighting(name: string | null): void {
  matchLighting = name;
}

/**
 * The battle's light for a lab or match URL.
 * Precedence: `?preset=` (and optional `?sun=`) capture override, then the
 * match setting (`?light=` or a saved replay's resolved look), then Daylight.
 */
export function gameLightFor(search: string): LightPresentation {
  const params = new URLSearchParams(search);
  const preset = params.get("preset");
  let light: LightPresentation = gameLight;
  let selected: string | null = null;
  const seed = params.get("seed") ?? "1";
  if (preset !== null) {
    light = lightForPreset(preset);
    selected = preset;
  } else {
    const parsed = parseLighting(params.get("light"));
    if (typeof parsed !== "string") throw new Error(parsed.error);
    const choice = matchLighting ?? parsed;
    const resolved = resolveLighting(choice, seed);
    if (resolved !== DEFAULT_LIGHTING) {
      light = lightForPreset(resolved);
    }
    selected = resolved;
  }
  light = lightWithMapSeed(light, seed);
  const sun = params.get("sun");
  if (sun !== null && Number.isFinite(Number(sun))) {
    light = { ...light, sun_elevation: Number(sun) };
  }
  if (
    preset !== null ||
    selected !== DEFAULT_LIGHTING ||
    (sun !== null && Number.isFinite(Number(sun)))
  ) {
    light_records(JSON.stringify(light));
  }
  return light;
}

/** The selected look's sun for fielding a lab battle (`?preset=`, `?light=`, `?sun=`). */
export function gameSun(): { azimuth: number; elevation: number } {
  const light = gameLightFor(typeof window === "undefined" ? "" : window.location.search);
  return { azimuth: light.sun_azimuth, elevation: light.sun_elevation };
}
