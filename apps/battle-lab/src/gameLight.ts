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
  const params = new URLSearchParams(search);
  const sun = params.get("sun");
  const gradeEnabled = params.get("capture-grade") !== "false";

  let light = gameLight;

  if (sun !== null && Number.isFinite(Number(sun))) {
    light = { ...light, sun_elevation: Number(sun) };
  }

  if (!gradeEnabled) {
    light = { ...light, grade: { ...light.grade, enabled: false } };
  }

  return validateLight(light);
}
