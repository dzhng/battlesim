import { light_records } from "@wasm/renderer_browser";
// The battle's light, from `fixtures/presentation.json`. Every
// lab route draws under it, as every lab reuses the game's numbers.
import { lightForPreset, presentationSettings } from "@web/presentationSettings";
import { type LightPresentation } from "@packages/battle-renderer/src/light/sceneLight";

export const gameLight: LightPresentation = presentationSettings.light;

/** The battle's light for a lab URL: `?preset=<name>` selects a fixture
 *  lighting look (`day`, `golden_hour`), and `?sun=<elevation, radians>`
 *  then lowers or raises the sun on top of that look. Lab only: the game
 *  draws the selected fixture preset. */
export function gameLightFor(search: string): LightPresentation {
  const params = new URLSearchParams(search);
  const preset = params.get("preset");
  let light: LightPresentation = gameLight;
  if (preset !== null) {
    light = lightForPreset(preset);
  }
  const sun = params.get("sun");
  if (sun !== null && Number.isFinite(Number(sun))) {
    light = { ...light, sun_elevation: Number(sun) };
  }
  if (preset !== null || (sun !== null && Number.isFinite(Number(sun)))) {
    light_records(JSON.stringify(light));
  }
  return light;
}

/** The selected look's sun for fielding a lab battle (`?preset=`, `?sun=`). */
export function gameSun(): { azimuth: number; elevation: number } {
  const light = gameLightFor(typeof window === "undefined" ? "" : window.location.search);
  return { azimuth: light.sun_azimuth, elevation: light.sun_elevation };
}
