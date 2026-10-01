// The battle's light, from the one fixture owner's presentation block. Every
// lab route draws under it, as every lab reuses the game's numbers.
import game from "@fixtures/game.json";
import {
  validateLight,
  type LightPresentation,
} from "@packages/battle-renderer/src/light/sceneLight";

export const gameLight: LightPresentation = validateLight(
  game.presentation.light as unknown as LightPresentation,
);

/** The battle's light with the sun set at `?sun=<elevation, radians>` in a
 *  lab URL (a dusk capture: the same fixture light, the sun low), else the
 *  fixture's. Lab only: the game draws the fixture's. */
export function gameLightFor(search: string): LightPresentation {
  const sun = new URLSearchParams(search).get("sun");
  if (sun === null || !Number.isFinite(Number(sun))) return gameLight;
  return validateLight({ ...gameLight, sun_elevation: Number(sun) });
}
