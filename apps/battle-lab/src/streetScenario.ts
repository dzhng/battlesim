// The street shared by the fog labs: the street test map's saved encounter
// `street` (`fixtures/maps/street/encounters/street.json`). Blue's nine units
// stand in and around the street; red holds it, its rifle squads garrisoning
// the three buildings under the encounter's defender policy.
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { savedBattle } from "./savedMaps";
import type { GameRules } from "@web/battle/catalog/compose";
import { useSessionCatalog } from "@web/battle/catalog/context";
import { useBuiltScenario } from "./useBuiltScenario";
import { gameCamera } from "./gameCamera";

export const STREET_SEED = 20260925;

/** ARMAPHRACT-like oblique over the street (spike 02's `street-oblique`). */
export const STREET_CAMERA: Camera3DParams = {
  target: [1030, 812, 0],
  distance: 150,
  pitch: 0.9076,
  yaw: 3.752,
  ...gameCamera.lens,
};

/** The street's scenario JSON, as `useBuiltScenario` reports it. */
export function useStreetScenario() {
  const { rules } = useSessionCatalog();
  return useBuiltScenario("street", () => buildStreetScenario(rules));
}

/** The street encounter on its test map, under `rules`. */
export async function buildStreetScenario(rules: GameRules) {
  return (await savedBattle("street", "street", rules)).scenario;
}
