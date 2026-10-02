// /lab/facade: what a facade material can say beyond "opaque", on the facade
// lab's kit (`packages/scene-assets/blender/city/facade_lab.py`), with no
// battle and no map: its modules stand on a plain ground at three stations.
//
// - cutouts: a grille and a perforated sheet as fence panels, turned so
//   their shadows fall where they can be read;
// - glass: free-standing panes, each half across the next behind it, and one
//   seen from its back;
// - facade: a block three bays wide and three floors high, a window in every
//   bay (shops at the ground), a guard over some.
//
// `?blocks=<n>` stands that many more blocks in a field behind (a dense view
// for the paired cost), `?fog=1` puts an eye before each station whose sight
// ends across it, `?azimuth=` and `?sun=` turn and raise the sun, and
// `?order=reversed` hands the frame the same modules in the opposite order.
import { useCallback, useMemo, useRef, useState } from "react";
import type { FogInput } from "@packages/battle-renderer/src/frame/fogInputs";
import { validateLight } from "@packages/battle-renderer/src/light/sceneLight";
import type { ModelInstance } from "@packages/battle-renderer/src/models/modelInstances";
import type { WorldLayers } from "@packages/battle-renderer/src/scene";
import { CameraController, type CameraPose } from "@packages/renderer-core/src/cameraController";
import game from "@fixtures/game.json";
import { useFeed } from "../feed";
import { useGameAppearances } from "../gameAppearances";
import { gameCamera } from "../gameCamera";
import { gameLight } from "../gameLight";
import { LabViewport, type ViewportPilot } from "../LabViewport";
import { benchGround, flatTerrain } from "../workbench/benchWorld";

const KIT = "city_kit_facade_lab";
const NONE = new Float32Array(0);
/** A bay's width and a floor's height, as the kit's script has them. */
const BAY_M = 3;
const FLOOR_M = 3;
const SILL_M = 0.95;
const BLOCK_BAYS = 3;
const BLOCK_FLOORS = 3;
/** The wall colour a block's tint-masked plaster takes (linear). */
const WALLS: [number, number, number][] = [
  [0.81, 0.72, 0.55],
  [0.72, 0.56, 0.45],
  [0.56, 0.62, 0.5],
  [0.6, 0.6, 0.57],
];
/** The fence panels' turn: three quarters on to the fixture's sun, so their
 *  shadows fall clear of them. */
const PANEL_YAW = 0.6;
/** Where each station stands, and how its camera looks at it (the eye is
 *  south of a facade, looking north, at yaw −π/2). */
const STATIONS = {
  cutouts: { at: [0, 0] as [number, number], yaw: -Math.PI / 2 + PANEL_YAW },
  glass: { at: [60, 0] as [number, number], yaw: -Math.PI / 2 },
  facade: { at: [120, 0] as [number, number], yaw: -Math.PI / 2 - 0.35 },
};
type Station = keyof typeof STATIONS;
/** The distances a station is seen from: close, the default camera, far. */
const DISTANCES = { close: 25, tactical: 65, far: 250 };
type Distance = keyof typeof DISTANCES;
/** The field of blocks behind the stations: where it starts, and its pitch. */
const FIELD = { at: [0, 60], pitch: [14, 12], columns: 24 };

interface Placement {
  module: string;
  x: number;
  y: number;
  z: number;
  yaw: number;
  tint?: [number, number, number];
}

/** A block at (x, y): its shell, a bay a floor in every bay of the lattice,
 *  and a guard over the windows `guarded` picks. */
function block(
  x: number,
  y: number,
  tint: [number, number, number],
  guarded: (bay: number, floor: number) => boolean,
): Placement[] {
  const out: Placement[] = [{ module: "block_shell", x, y, z: 0, yaw: 0, tint }];
  for (let floor = 0; floor < BLOCK_FLOORS; floor++)
    for (let bay = 0; bay < BLOCK_BAYS; bay++) {
      const at = { x: x + (bay - (BLOCK_BAYS - 1) / 2) * BAY_M, y, z: floor * FLOOR_M, yaw: 0 };
      out.push({ module: floor === 0 ? "bay_shop" : "bay_window", ...at, tint });
      if (floor > 0 && guarded(bay, floor))
        out.push({ module: "window_grille", ...at, z: at.z + SILL_M });
    }
  return out;
}

function layout(blocks: number): Placement[] {
  const cut = STATIONS.cutouts.at;
  const side = [Math.cos(PANEL_YAW), Math.sin(PANEL_YAW)];
  const out: Placement[] = [
    {
      module: "grille_panel",
      x: cut[0] - 2 * side[0],
      y: cut[1] - 2 * side[1],
      z: 0,
      yaw: PANEL_YAW,
    },
    {
      module: "sheet_panel",
      x: cut[0] + 2 * side[0],
      y: cut[1] + 2 * side[1],
      z: 0,
      yaw: PANEL_YAW,
    },
    // Three panes stepping back and across, so each is half behind the last,
    // and a fourth turned round.
    ...[0, 1, 2].map((i) => ({
      module: "pane",
      x: STATIONS.glass.at[0] - 1.5 + i * 0.75,
      y: STATIONS.glass.at[1] + i * 1.5,
      z: 0,
      yaw: 0,
    })),
    { module: "pane", x: STATIONS.glass.at[0] + 2.5, y: STATIONS.glass.at[1], z: 0, yaw: Math.PI },
    ...block(...STATIONS.facade.at, WALLS[0], (bay, floor) => bay === floor - 1),
  ];
  for (let i = 0; i < blocks; i++)
    out.push(
      ...block(
        FIELD.at[0] + (i % FIELD.columns) * FIELD.pitch[0],
        FIELD.at[1] + Math.floor(i / FIELD.columns) * FIELD.pitch[1],
        WALLS[i % WALLS.length],
        () => true,
      ),
    );
  return out;
}

/** A point of a placed module's own frame, in the world. */
function worldOf(p: Placement, local: readonly [number, number, number]): [number, number, number] {
  const [c, s] = [Math.cos(p.yaw), Math.sin(p.yaw)];
  return [p.x + local[0] * c - local[1] * s, p.y + local[0] * s + local[1] * c, p.z + local[2]];
}

/** Sight over the flat ground, from an eye 40 m before each station: it ends
 *  a metre and a half behind the station's middle. */
function fogInput(): FogInput {
  const n = 64;
  const world = {
    nx: n,
    ny: n,
    spacing: 8,
    pageSize: 16,
    minHeight: 0,
    pageIds: new Uint32Array(0),
    heights: NONE,
    foliage: Float32Array.of(n, n, 8),
    targetHeightM: game.sensors.fog_target_height_m,
    foliageFullBlock: game.sensors.foliage_full_block,
  };
  const eyes = (Object.keys(STATIONS) as Station[]).map((id) => {
    const { at, yaw } = STATIONS[id];
    const range = 40;
    return {
      key: id,
      position: [
        at[0] + Math.cos(yaw) * (range - 1.5),
        at[1] + Math.sin(yaw) * (range - 1.5),
        1.7,
      ] as [number, number, number],
      forward: yaw + Math.PI,
      shape: { front: 1, side: 1, rear: 1 },
      range,
    };
  });
  return { world, sight: { eyes, occluders: [] } };
}

export default function Facade() {
  const query = useMemo(() => new URLSearchParams(window.location.search), []);
  const blocks = Math.max(0, Math.floor(Number(query.get("blocks") ?? 0)) || 0);
  const light = useMemo(() => {
    const number = (name: string, or: number) =>
      query.get(name) !== null && Number.isFinite(Number(query.get(name)))
        ? Number(query.get(name))
        : or;
    return validateLight({
      ...gameLight,
      sun_azimuth: number("azimuth", gameLight.sun_azimuth),
      sun_elevation: number("sun", gameLight.sun_elevation),
    });
  }, [query]);
  const installed = useGameAppearances();
  // The kit alone: nothing else is drawn here.
  const appearances = useMemo(
    () =>
      installed && {
        ...installed,
        appearances: new Map([...installed.appearances].filter(([name]) => name === KIT)),
        templates: undefined,
      },
    [installed],
  );
  const placed = useMemo(
    () => (query.get("order") === "reversed" ? layout(blocks).reverse() : layout(blocks)),
    [blocks, query],
  );
  const models = useMemo<ModelInstance[]>(
    () =>
      placed.map((p) => ({
        appearance: KIT,
        x: p.x,
        y: p.y,
        z: p.z,
        yaw: p.yaw,
        pose: { kind: "static", state: p.module },
        ...(p.tint ? { tint: p.tint } : {}),
      })),
    [placed],
  );
  const world = useMemo<WorldLayers>(
    () => ({
      terrain: flatTerrain(benchGround(false)),
      props: NONE,
      water: NONE,
      structures: [],
      scenery: null,
      grass: null,
    }),
    [],
  );
  const worldFeed = useFeed<WorldLayers | null>(world);
  const fog = useMemo(() => (query.get("fog") === "1" ? fogInput() : null), [query]);
  const fogFeed = useFeed(fog);

  const stations = useMemo(() => {
    const rig = new CameraController(gameCamera.config);
    const pose = (id: Station, distance: number): CameraPose => ({
      target: STATIONS[id].at,
      distance,
      yaw: STATIONS[id].yaw,
      pitch: rig.pitchAt(distance),
    });
    return { pose };
  }, []);
  const [station, setStation] = useState<[Station, Distance]>(["facade", "tactical"]);
  const cut = useRef<CameraPose | null>(null);
  const [pilot] = useState<ViewportPilot>(() => ({
    pose() {
      const once = cut.current;
      cut.current = null;
      return once;
    },
  }));
  const stand = useCallback(
    (id: Station, distance: Distance) => {
      cut.current = stations.pose(id, DISTANCES[distance]);
      setStation([id, distance]);
    },
    [stations],
  );

  const diagnostics = useMemo(
    () => ({
      surfaceZ: () => 0,
      stations: () => Object.keys(STATIONS),
      distances: () => DISTANCES,
      /** Cut the camera to a station at a named distance, as the rig places it. */
      stand,
      /** Where each module stands. */
      layout: () => placed,
      light: () => ({ azimuth: light.sun_azimuth, elevation: light.sun_elevation }),
      /** World points of a placement of `module` (the `nth` the layout
       *  stands), from points in its own frame. */
      points: (module: string, locals: [number, number, number][], nth = 0) => {
        const p = layout(blocks).filter((q) => q.module === module)[nth];
        return p ? locals.map((l) => worldOf(p, l)) : null;
      },
    }),
    [placed, blocks, light, stand],
  );

  if (!appearances) return null;
  const opening = stations.pose(...(["facade", DISTANCES.tactical] as const));
  return (
    <>
      <LabViewport
        fixture="facade"
        world={worldFeed}
        fog={fogFeed}
        light={light}
        appearances={appearances}
        models={models}
        initialCamera={{
          ...gameCamera.opening(),
          ...opening,
          target: [opening.target[0], opening.target[1], 0],
        }}
        groundAt={() => 0}
        pilot={pilot}
        diagnostics={diagnostics}
      />
      <aside className="hud-panel lab-panel" data-testid="facade-panel">
        <strong>Facade</strong>
        <div className="lab-hint">
          Cutout, glass and room surfaces on the facade lab's kit. WASD/arrows pan · Q/E turn ·
          wheel zoom
        </div>
        {(Object.keys(STATIONS) as Station[]).map((id) => (
          <div className="lab-row" key={id}>
            <span>{id}</span>
            {(Object.keys(DISTANCES) as Distance[]).map((distance) => (
              <button
                key={distance}
                type="button"
                aria-pressed={station[0] === id && station[1] === distance}
                onClick={() => stand(id, distance)}
              >
                {distance}
              </button>
            ))}
          </div>
        ))}
      </aside>
    </>
  );
}
