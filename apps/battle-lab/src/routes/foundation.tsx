import { useMemo, useState } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { MeshBuilder, type Rgba } from "@packages/battle-renderer/src/mesh";
import type { SceneInstance, WorldLayers } from "@packages/battle-renderer/src/scene";
import { terrainSurface } from "@packages/battle-renderer/src/terrain/terrainSurface";
import { LabViewport } from "../LabViewport";
import { villageBiome } from "../villageBiome";

// Render-only fixture: a raised ground patch and hand-placed proxies. It has no
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
    roads: none,
    roadStride: 5,
    forests: none,
    water: none,
    buildings: [],
    footprints: none,
  };
  return {
    // The checker patch is the tint alone: no grass over it.
    terrain: terrainSurface(mesh.build(), site, villageBiome, null),
    props: none,
    translucent: none,
    scenery: null,
  };
}

const BLUE = [0.55, 0.7, 1.0] as const;

const onPatch = (inst: Omit<SceneInstance, "z">): SceneInstance => ({
  ...inst,
  z: patchHeight(inst.x, inst.y),
});

const FOUNDATION_INSTANCES: SceneInstance[] = [
  onPatch({ kind: "tank", x: 0, y: 0, yaw: 0.35, color: BLUE }),
  // Crate the tank's barrel pierces: the depth-overlap check.
  onPatch({ kind: "box", x: 4.9, y: 1.9, yaw: 0.2, color: [1, 1, 1] }),
  ...[0, 1, 2, 3].map((i) =>
    onPatch({
      kind: "infantry",
      x: -6 + i * 2.2,
      y: -7 - (i % 2) * 1.5,
      yaw: -Math.PI / 2 + i * 0.9,
      color: BLUE,
    }),
  ),
  onPatch({ kind: "supply", x: 10, y: -12, yaw: Math.PI * 0.8, color: BLUE }),
];

const FOUNDATION_CAMERA: Camera3DParams = {
  target: [-2, 1, 0.5],
  distance: 38,
  pitch: 0.6,
  yaw: -1.1,
  fovY: 0.8,
  aspect: 1,
  near: 0.5,
};

export default function Foundation() {
  const world = useMemo(groundPatch, []);
  const [selected, setSelected] = useState(-1);
  const instances = useMemo(
    () => FOUNDATION_INSTANCES.map((inst, i) => ({ ...inst, highlight: i === selected })),
    [selected],
  );
  return (
    <>
      <LabViewport
        fixture="foundation"
        world={world}
        instances={instances}
        initialCamera={FOUNDATION_CAMERA}
        onPick={(pick) => pick.button === "left" && setSelected(pick.instance)}
      />
      <aside className="lab-panel" data-testid="foundation-panel">
        <strong>Foundation</strong>
        <div>
          Selected: {selected >= 0 ? `${FOUNDATION_INSTANCES[selected].kind} #${selected}` : "none"}
        </div>
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
