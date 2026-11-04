/** Offline admission for a self-contained scenario download. */
import type { PreparedWorld } from "./prepare";
import type { MapExtents } from "@packages/battle-renderer/src/worldMesh";
import {
  validateScenarioBundle,
  type ScenarioBundleIdentity,
  type ScenarioBundleWasm,
} from "../scenarioBundle";
import { composeScenarioCatalog } from "../catalog/compose";
import type { SessionCatalog } from "../catalog/compose";
import type { SideName } from "../sim/protocol";

export interface ScenarioBundlePreparationModule extends ScenarioBundleWasm {
  resolve_catalog(documents: string): string;
  scenario_bundle_details(bundle: string): string;
  scenario_bundle_controllers(bundle: string, active_human?: string): string;
  PreparedWorld: {
    from_bundle(bundle: string): PreparedWorld;
  };
}

export function scenarioBundleCatalog(
  scenario: string,
  wasm: Pick<ScenarioBundlePreparationModule, "resolve_catalog">,
): SessionCatalog {
  const parsed = JSON.parse(scenario) as { rules: import("../catalog/compose").ResolvedRules };
  return composeScenarioCatalog(parsed.rules, wasm.resolve_catalog);
}

export interface ScenarioBundlePreparationReport {
  identity: ScenarioBundleIdentity;
  size: [number, number];
  extents: MapExtents;
  counts: { buildings: number; parts: number; props: number; surfaces: number; forests: number };
  start: { at: [number, number]; yaw: number };
  town: [number, number] | null;
  assets: { id: string; media_type: string; bytes_base64: string }[];
  human_capable_players: string[];
  active_human_player: string | null;
  active_human_side: SideName | null;
}

export interface PreparedScenarioBundle {
  canonical: string;
  scenario: string;
  report: ScenarioBundlePreparationReport;
  world: PreparedWorld;
}

/**
 * Admit a downloaded bundle without map generation, fixture resolution, or
 * browser-side interpretation of physical map data. The WASM module returns
 * both the resolved scenario and the metadata used by the loading/camera UI.
 */
export function prepareScenarioBundle(
  wasm: ScenarioBundlePreparationModule,
  bundle: string,
  activeHuman?: string | null,
): PreparedScenarioBundle {
  const identity = validateScenarioBundle(wasm, bundle);
  const canonical = wasm.canonical_scenario_bundle(bundle);
  const details = JSON.parse(wasm.scenario_bundle_details(canonical)) as {
    scenario: string;
    identity: ScenarioBundleIdentity;
    size: [number, number];
    extents: MapExtents;
    counts: ScenarioBundlePreparationReport["counts"];
    start: ScenarioBundlePreparationReport["start"];
    town: [number, number] | null;
    assets: ScenarioBundlePreparationReport["assets"];
    human_capable_players: string[];
  };
  if (details.identity.sha256 !== identity.sha256)
    throw new Error("bundle identity changed during admission");
  const envelope = JSON.parse(canonical) as {
    scenario?: { players?: { id: string; side: SideName; human_capable?: boolean }[] };
  };
  const players = envelope.scenario?.players;
  const active =
    activeHuman === null
      ? null
      : (players?.find((player) => player.id === activeHuman && player.human_capable) ??
        players?.find((player) => player.human_capable) ??
        null);
  const scenario = JSON.parse(details.scenario) as {
    units?: { owner?: string; position: [number, number]; yaw: number }[];
  };
  const activeUnit = active?.id
    ? scenario.units?.find((unit) => unit.owner === active.id)
    : undefined;
  return {
    canonical,
    scenario: details.scenario,
    report: {
      identity: details.identity,
      size: details.size,
      extents: details.extents,
      counts: details.counts,
      start: activeUnit ? { at: activeUnit.position, yaw: activeUnit.yaw } : details.start,
      town: details.town,
      assets: details.assets,
      human_capable_players: details.human_capable_players,
      active_human_player: active?.id ?? null,
      active_human_side: active?.side ?? null,
    },
    world: wasm.PreparedWorld.from_bundle(canonical),
  };
}
