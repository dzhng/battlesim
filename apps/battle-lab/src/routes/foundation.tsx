import { useMemo, useState } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { MeshData } from "@packages/battle-renderer/src/proxies";
import type { SceneInstance } from "@packages/battle-renderer/src/scene";
import { LabViewport } from "../LabViewport";

// Render-only fixture: a raised ground patch and hand-placed proxies. It has no
// simulation meaning; authoritative terrain arrives with the world geometry owner.

const PATCH_HALF = 40;
const CELL = 2;

function patchHeight(x: number, y: number): number {
  return 2.5 * Math.exp(-((x + 18) ** 2 + (y - 16) ** 2) / 90);
}

function groundPatch(): MeshData {
  const positions: number[] = [],
    normals: number[] = [],
    colors: number[] = [];
  const vertex = (x: number, y: number) => {
    const z = patchHeight(x, y);
    const e = 0.01;
    const nx = -(patchHeight(x + e, y) - patchHeight(x - e, y)) / (2 * e);
    const ny = -(patchHeight(x, y + e) - patchHeight(x, y - e)) / (2 * e);
    const l = Math.hypot(nx, ny, 1);
    positions.push(x, y, z);
    normals.push(nx / l, ny / l, 1 / l);
    const checker = (Math.floor(x / 8) + Math.floor(y / 8)) & 1 ? 0.04 : 0;
    colors.push(0.36 + z * 0.04 + checker, 0.46 + z * 0.03 + checker, 0.3 + checker);
  };
  for (let x = -PATCH_HALF; x < PATCH_HALF; x += CELL) {
    for (let y = -PATCH_HALF; y < PATCH_HALF; y += CELL) {
      // Southwest-to-northeast diagonal, the project-wide triangulation.
      vertex(x, y);
      vertex(x + CELL, y);
      vertex(x + CELL, y + CELL);
      vertex(x, y);
      vertex(x + CELL, y + CELL);
      vertex(x, y + CELL);
    }
  }
  return {
    positions: Float32Array.from(positions),
    normals: Float32Array.from(normals),
    colors: Float32Array.from(colors),
  };
}

const BLUE = [0.55, 0.7, 1.0] as const;

const onPatch = (inst: Omit<SceneInstance, "z">): SceneInstance => ({
  ...inst,
  z: patchHeight(inst.x, inst.y),
});

export const FOUNDATION_INSTANCES: SceneInstance[] = [
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

export const FOUNDATION_CAMERA: Camera3DParams = {
  target: [-2, 1, 0.5],
  distance: 38,
  pitch: 0.6,
  yaw: -1.1,
  fovY: 0.8,
  aspect: 1,
  near: 0.5,
};

export default function Foundation() {
  const mesh = useMemo(groundPatch, []);
  const [selected, setSelected] = useState(-1);
  const instances = useMemo(
    () => FOUNDATION_INSTANCES.map((inst, i) => ({ ...inst, highlight: i === selected })),
    [selected],
  );
  return (
    <>
      <LabViewport
        fixture="foundation"
        mesh={mesh}
        instances={instances}
        initialCamera={FOUNDATION_CAMERA}
        onPick={setSelected}
      />
      <aside className="lab-panel" data-testid="foundation-panel">
        <strong>Foundation</strong>
        <div>
          Selected: {selected >= 0 ? `${FOUNDATION_INSTANCES[selected].kind} #${selected}` : "none"}
        </div>
        <div className="lab-hint">Click select · middle-drag orbit · WASD pan · wheel zoom</div>
        <button type="button" onClick={() => window.__lab?.reset?.()}>
          Reset camera
        </button>
      </aside>
    </>
  );
}
