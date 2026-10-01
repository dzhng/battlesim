import { useMemo, useState } from "react";
import game from "@fixtures/game.json";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { MeshBuilder, type Rgba } from "@packages/battle-renderer/src/mesh";
import type { SceneInstance, WorldLayers } from "@packages/battle-renderer/src/scene";
import { terrainSurface } from "@packages/battle-renderer/src/terrain/terrainSurface";
import { REST_ARTICULATION } from "@packages/scene-assets/src/articulation";
import type { ModelInstance } from "@packages/battle-renderer/src/models/modelInstances";
import { bodyBox, proxyPickBox, type PickBox } from "@packages/battle-renderer/src/picking";
import { LabViewport } from "../LabViewport";
import { AppearanceCatalog } from "@packages/scene-assets/src/appearanceCatalog";
import { gameBiome } from "../gameBiome";
import { useGameAppearances } from "../gameAppearances";
import { useFeed } from "../feed";
import { gameCamera } from "../gameCamera";

// Render-only fixture: a raised ground patch, a tank and a truck drawn as their
// appearances, and hand-placed proxies (soldiers, a crate). It has no
// simulation meaning; authoritative terrain arrives with the world geometry owner.

const PATCH_HALF = 40;
const CELL = 2;

function patchHeight(x: number, y: number): number {
  return 2.5 * Math.exp(-((x + 18) ** 2 + (y - 16) ** 2) / 90);
}

/** The patch's own checker, fully over the biome (the tint's alpha is its weight). */
function groundPatch(): WorldLayers {
  const mesh = new MeshBuilder();
  const vertex = (x: number, y: number): [number, number, number] => [x, y, patchHeight(x, y)];
  const color = (x: number, y: number): Rgba => {
    const z = patchHeight(x, y);
    const checker = (Math.floor(x / 8) + Math.floor(y / 8)) & 1 ? 0.04 : 0;
    return [0.36 + z * 0.04 + checker, 0.46 + z * 0.03 + checker, 0.3 + checker, 1];
  };
  for (let x = -PATCH_HALF; x < PATCH_HALF; x += CELL) {
    for (let y = -PATCH_HALF; y < PATCH_HALF; y += CELL) {
      // Southwest-to-northeast diagonal, the project-wide triangulation.
      const [sw, se, ne, nw] = [
        [x, y],
        [x + CELL, y],
        [x + CELL, y + CELL],
        [x, y + CELL],
      ] as const;
      mesh.shadedTriangle(
        vertex(...sw),
        vertex(...se),
        vertex(...ne),
        color(...sw),
        color(...se),
        color(...ne),
      );
      mesh.shadedTriangle(
        vertex(...sw),
        vertex(...ne),
        vertex(...nw),
        color(...sw),
        color(...ne),
        color(...nw),
      );
    }
  }
  const none = new Float32Array(0);
  const site = {
    map: [-PATCH_HALF, -PATCH_HALF, PATCH_HALF, PATCH_HALF] as const,
    gridM: CELL,
    surfaceStrokes: none,
    surfaceStrokeStride: 6,
    surfaceRuns: none,
    surfaceRunStride: 4,
    surfaceTriangles: none,
    surfaceBoundaries: none,
    surfaceTriangleStride: 7,
    surfaceBoundaryStride: 5,
    forests: none,
    forestShapes: [],
    rivers: none,
    riverRuns: none,
    riverRunStride: 4,
    buildings: [],
    footprints: none,
  };
  return {
    // The checker patch is the tint alone: no grass over it.
    terrain: terrainSurface(mesh.build(), site, gameBiome, null),
    props: none,
    water: none,
    structures: [],
    scenery: null,
    grass: null,
  };
}

const BLUE = [0.55, 0.7, 1.0] as const;

/** One placed thing: a vehicle (a model) or a proxy (a soldier, the crate). */
interface Placed {
  kind: "tank" | "supply" | "box" | "infantry";
  x: number;
  y: number;
  z: number;
  yaw: number;
}

const onPatch = (p: Omit<Placed, "z">): Placed => ({ ...p, z: patchHeight(p.x, p.y) });

const FOUNDATION: Placed[] = [
  onPatch({ kind: "tank", x: 0, y: 0, yaw: 0.35 }),
  // A crate the tank's gun reaches over: the depth-overlap check.
  onPatch({ kind: "box", x: 6.4, y: 2.5, yaw: 0.2 }),
  ...[0, 1, 2, 3].map((i) =>
    onPatch({
      kind: "infantry",
      x: -6 + i * 2.2,
      y: -7 - (i % 2) * 1.5,
      yaw: -Math.PI / 2 + i * 0.9,
    }),
  ),
  onPatch({ kind: "supply", x: 10, y: -12, yaw: Math.PI * 0.8 }),
];
/** A placed unit type's hull; none for the soldier and the crate. */
const hullOf = (kind: string) => (UNITS.has(kind) ? UNITS.hull(kind) : null);

/** Soldiers and vehicles are picked by the simulation's boxes; the crate by its own. */
const FOUNDATION_TARGETS: PickBox[] = FOUNDATION.map((p) =>
  p.kind === "box"
    ? proxyPickBox({ ...p, kind: "box", color: [1, 1, 1] })
    : { x: p.x, y: p.y, z: p.z, yaw: p.yaw, ...bodyBox(game.physics, hullOf(p.kind)) },
);

const FOUNDATION_CAMERA: Camera3DParams = {
  target: [-2, 1, 0.5],
  distance: 38,
  pitch: 0.6,
  yaw: -1.1,
  ...gameCamera.lens,
  near: 0.5,
};

export default function Foundation() {
  const world = useMemo(groundPatch, []);
  const worldFeed = useFeed(world);
  const appearances = useGameAppearances();
  const [selected, setSelected] = useState(-1);
  const instances = useMemo<SceneInstance[]>(
    () =>
      FOUNDATION.flatMap((p, i) =>
        p.kind === "box" || p.kind === "infantry"
          ? [
              {
                ...p,
                kind: p.kind,
                color: p.kind === "box" ? ([1, 1, 1] as const) : BLUE,
                highlight: i === selected,
              },
            ]
          : [],
      ),
    [selected],
  );
  const models = useMemo<ModelInstance[]>(() => {
    if (!appearances) return [];
    const catalog = new AppearanceCatalog(appearances, UNITS);
    return FOUNDATION.flatMap((p, i) => {
      const resolved = hullOf(p.kind) ? catalog.resolve(p.kind, "blue") : null;
      return resolved
        ? [
            {
              appearance: resolved.appearance,
              tint: resolved.tint,
              x: p.x,
              y: p.y,
              z: p.z,
              yaw: p.yaw,
              pose: { kind: "articulated" as const, articulation: { ...REST_ARTICULATION } },
              highlight: i === selected,
            },
          ]
        : [];
    });
  }, [appearances, selected]);
  const diagnostics = useMemo(() => ({ placed: FOUNDATION }), []);
  if (!appearances) return null;
  return (
    <>
      <LabViewport
        fixture="foundation"
        world={worldFeed}
        instances={instances}
        appearances={appearances}
        models={models}
        picks={FOUNDATION_TARGETS}
        initialCamera={FOUNDATION_CAMERA}
        onPick={(pick) => pick.button === "left" && setSelected(pick.instance)}
        diagnostics={diagnostics}
      />
      <aside className="hud-panel lab-panel" data-testid="foundation-panel">
        <strong>Foundation</strong>
        <div>Selected: {selected >= 0 ? `${FOUNDATION[selected].kind} #${selected}` : "none"}</div>
        <div className="lab-hint">
          Click select · WASD/arrows/screen edge pan · Q/E turn · middle‑drag orbit · wheel zoom
        </div>
        <button type="button" onClick={() => window.__lab?.reset?.()}>
          Reset camera
        </button>
      </aside>
    </>
  );
}
