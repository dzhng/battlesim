import { useCallback, useMemo, useState } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh";
import { OPEN } from "@packages/battle-renderer/src/terrain/terrainSurface";
import { buildWorldLayers } from "@packages/battle-renderer/src/worldMesh";
import type { OwnUnitView } from "@web/battle/sim/observation";
import type { Order } from "@web/battle/sim/protocol";
import { AckLog } from "../AckLog";
import { orderLayer } from "../battleOverlay";
import { LabViewport } from "../LabViewport";
import { SavedEncounter, type SavedBattle } from "../savedMaps";
import { useBattleSession } from "../useBattleSession";
import { useFeed } from "../feed";
import { gameBiome } from "../gameBiome";
import { gameCamera } from "../gameCamera";

// The river map's saved encounter (`fixtures/maps/river/encounters/river.json`):
// a rifle squad on the country road south of its bridge, and a tank ahead of
// it that leads over the bridge.

/** Where units are sent: named so the scene and the panel agree. */
const STATIONS = {
  /** On the road past the bridge. */
  north: [60, 310],
  /** On the dirt track beside the 30 m stretch. */
  track: [450, 176],
  /** The middle of the 30 m stretch: water. */
  midstream: [500, 225.5],
} as const satisfies Record<string, readonly [number, number]>;

/** What the ground is drawn as: the biome; what blocks (red); or one tint over
 *  everything, so only the ground's shading shows. */
type GroundView = "surface" | "traversal" | "shading";
/** Where a vertex's tint sits: after its position and normal. */
const VERTEX_COLOUR = 6;

const SEED = 4;

const RIVER_CAMERA: Camera3DParams = {
  target: [300, 240, 0],
  distance: 430,
  pitch: 0.95,
  yaw: -Math.PI / 2,
  ...gameCamera.lens,
};

const send = (units: number[], gesture: number, goal: readonly [number, number]): Order => ({
  kind: "move",
  units,
  gesture,
  goal: [goal[0], goal[1]],
  route: "shortest",
});

/** Reference commands the lab can send, exactly as a player would. */
const DEMOS: Record<string, (own: OwnUnitView[]) => Order[]> = {
  "Over the bridge": (own) =>
    own.map((u, k) => send([u.id], 9101 + k, [STATIONS.north[0], STATIONS.north[1] + 30 * k])),
  "Along the south bank": (own) => [
    send(
      own.map((u) => u.id),
      9110,
      STATIONS.track,
    ),
  ],
  // Each on its own: a group's places would straddle the water, one bank each.
  "Into the river": (own) => own.map((u, k) => send([u.id], 9120 + k, STATIONS.midstream)),
};

export default function River() {
  return (
    <SavedEncounter map="river" encounter="river">
      {(battle) => <RiverLab battle={battle} />}
    </SavedEncounter>
  );
}

function RiverLab({ battle }: { battle: SavedBattle }) {
  const session = useBattleSession({ ...battle, seed: SEED });
  const { world, meshes, sim, control, surfaceZ } = session;
  const [view, setView] = useState<GroundView>("surface");
  const drawn = useMemo(() => {
    if (view === "surface" || !world) return meshes;
    const layers = buildWorldLayers(world.exports, world.layout, gameBiome, "traversal");
    if (view === "traversal") return layers;
    // One tint over every triangle: what is left is the ground's shading.
    const mesh = layers.terrain.mesh.slice();
    for (let v = 0; v < mesh.length; v += VERTEX_FLOATS) mesh.set(OPEN, v + VERTEX_COLOUR);
    return { ...layers, terrain: { ...layers.terrain, mesh } };
  }, [view, world, meshes]);
  const worldFeed = useFeed(drawn);
  const { observation } = sim;

  const orders = useMemo(() => {
    if (!world || !observation) return undefined;
    return orderLayer(observation, control.selected, session.revealed, surfaceZ);
  }, [world, observation, control.selected, session.revealed, surfaceZ]);
  const overlayFeed = useFeed(orders);

  const runDemo = useCallback(
    async (name: string) => {
      if (!observation) return;
      const steps = DEMOS[name](observation.own);
      control.setSelected([...new Set(steps.flatMap((s) => ("units" in s ? s.units : [])))]);
      for (const step of steps) await control.issue(step, false);
    },
    [observation, control],
  );

  // Lab-only probes for the scene harness; rebuilt each render.
  const diagnostics = {
    ...session.probes,
    demo: (name: string) => runDemo(name),
    stations: STATIONS,
    setGroundView: setView,
    exports: () => world?.exports,
    layout: () => world?.layout,
    /** The authoritative surface at a point: its kind and whether a mover may stand there. */
    surfaceAt: (x: number, y: number) => {
      const s = world?.view.surface_at(x, y);
      return s?.length
        ? { z: s[0], slope: s[4], kind: world!.layout.surfaceKinds[s[5]], traversable: s[7] === 1 }
        : null;
    },
    groundHeight: (x: number, y: number) => world?.view.height_at(x, y),
  };

  if (!drawn) return null;
  return (
    <>
      <LabViewport
        fixture="river"
        world={worldFeed}
        structures={view === "surface" ? session.structures : undefined}
        buildings={view === "surface" ? session.buildingsFeed : undefined}
        overlay={overlayFeed}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={RIVER_CAMERA}
        onPick={session.onPick}
        onBox={session.onBox}
        onReady={session.onReady}
        diagnostics={diagnostics}
      />
      <aside className="hud-panel lab-panel" data-testid="river-panel">
        <strong>River</strong>
        <div className="lab-hint">
          A river is water wherever a point lies within half its width of its rounded centreline: 12
          m at the bridge, 30 m in the east. Nothing crosses it but by the deck.
        </div>
        <div>
          Tick {observation?.tick ?? 0} · {sim.status.status}
        </div>
        <div className="lab-row">
          {Object.keys(DEMOS).map((name) => (
            <button key={name} type="button" onClick={() => void runDemo(name)}>
              {name}
            </button>
          ))}
          <button type="button" onClick={sim.reset}>
            Reset
          </button>
        </div>
        <label>
          <input
            type="checkbox"
            checked={view === "traversal"}
            onChange={(e) => setView(e.target.checked ? "traversal" : "surface")}
          />{" "}
          Traversable ground (red: blocked)
        </label>
        <ul className="lab-log" data-testid="selection">
          {control.selectedUnits.length === 0 && <li>No unit selected</li>}
          {control.selectedUnits.map((u) => (
            <li key={u.id}>
              {u.kind} #{u.id}:{" "}
              {u.state === "route_blocked"
                ? `route blocked — no way to (${u.goal!.map((v) => v.toFixed(0)).join(", ")})`
                : u.state}
            </li>
          ))}
        </ul>
        <AckLog acks={control.acks} />
      </aside>
    </>
  );
}
