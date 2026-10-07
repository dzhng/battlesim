// Spike 02's village street, shared by the fog labs: blue's nine units stand
// in and around the street, red holds the village as authored (its rifle
// squads garrison the three buildings). Same unit count and order as the
// village, so red's ids and garrisons are unchanged.
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { villageScenario } from "./savedMaps";
import type { GameRules } from "@web/battle/catalog/compose";
import { useSessionCatalog } from "@web/battle/catalog/context";
import { useBuiltScenario } from "./useBuiltScenario";
import { gameCamera } from "./gameCamera";
import type { Wasm } from "@web/battle/sim/module";

/** Blue's nine units, in the village's blue order. */
const STREET: { kind: string; position: [number, number]; yaw: number }[] = [
  { kind: "test_recon", position: [1004, 788], yaw: 0 },
  { kind: "test_rifle", position: [1012, 842], yaw: 0 },
  { kind: "test_rifle", position: [930, 800], yaw: 0 },
  { kind: "test_rifle", position: [905, 745], yaw: 0 },
  { kind: "test_tank", position: [942, 826], yaw: 0.15 },
  { kind: "test_tank", position: [870, 790], yaw: 0.3 },
  { kind: "test_at", position: [950, 715], yaw: 0 },
  { kind: "test_supply", position: [820, 790], yaw: 0 },
  { kind: "test_jeep", position: [855, 812], yaw: 0 },
];
export const STREET_SEED = 20260925;

/** ARMAPHRACT-like oblique over the street (spike 02's `street-oblique`). */
export const STREET_CAMERA: Camera3DParams = {
  target: [1030, 812, 0],
  distance: 150,
  pitch: 0.9076,
  yaw: 3.752,
  ...gameCamera.lens,
};

type UnitSetup = { side: string; kind: string; position: [number, number]; yaw: number };

/** The street's scenario JSON, as `useBuiltScenario` reports it. */
export function useStreetScenario(fixture: string) {
  const { rules } = useSessionCatalog();
  return useBuiltScenario(fixture, (wasm, fixture) => buildStreetScenario(wasm, fixture, rules));
}

export async function buildStreetScenario(wasm: Wasm, fixture: string, rules: GameRules) {
  const s = JSON.parse(await villageScenario(wasm, fixture, "ordinary", rules)) as {
    units: UnitSetup[];
  };
  const blue = s.units.filter((u) => u.side === "blue");
  if (blue.length !== STREET.length) throw new Error("the street places the village's blue units");
  let b = 0;
  s.units = s.units.map((u) => (u.side === "blue" ? { ...u, ...STREET[b++] } : u));
  return JSON.stringify(s);
}
