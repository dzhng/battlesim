import { light_records } from "@wasm/renderer_browser";
// The battle's light, from `fixtures/presentation.json`. Every
// lab route draws under it, as every lab reuses the game's numbers.
import { presentationSettings } from "@web/presentationSettings";
import { type LightPresentation } from "@packages/battle-renderer/src/light/sceneLight";

export const gameLight: LightPresentation = presentationSettings.light;

/** The battle's light with the sun set at `?sun=<elevation, radians>` in a
 *  lab URL (a dusk capture: the same fixture light, the sun low), else the
 *  fixture's. Lab only: the game draws the fixture's. */
export function gameLightFor(search: string): LightPresentation {
  const params = new URLSearchParams(search);
  const sun = params.get("sun");
  if (sun === null || !Number.isFinite(Number(sun))) return gameLight;
  const light = { ...gameLight, sun_elevation: Number(sun) };
  light_records(JSON.stringify(light));
  return light;
}
