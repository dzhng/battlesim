import { expect, test } from "vitest";
import { appendRemoteTerrain } from "../../apps/battle-lab/src/benchmark/matchedWorld";
import type { WorldExports } from "../../packages/battle-renderer/src/worldMesh";

test.each([0, 100])(
  "remote growth at elevation %s preserves local words and identities",
  (elevation) => {
    const local = {
      positions: new Float32Array([-0, 0, 2, 20, 0, 3, 0, 20, 4]),
      indices: new Uint32Array([0, 1, 2]),
      triangleSurfaces: new Uint8Array([3, 1]),
      extents: { physical: [0, 0, 20, 20], playable: [0, 0, 20, 20], rendered: [0, 0, 20, 20] },
      props: new Float32Array([7]),
      buildings: { buildings: [{ owner: 42 }] },
    } as unknown as WorldExports;
    let previous = local.positions.length;
    for (const copies of [1, 4, 16]) {
      const arm = appendRemoteTerrain(local, copies, elevation);
      expect(arm.positions.slice(0, local.positions.length)).toEqual(local.positions);
      expect(arm.indices.slice(0, 3)).toEqual(local.indices);
      expect(arm.triangleSurfaces.slice(0, 2)).toEqual(local.triangleSurfaces);
      expect(arm.props).toBe(local.props);
      expect(arm.buildings).toBe(local.buildings);
      expect(arm.positions.length).toBeGreaterThan(previous);
      expect(arm.positions[local.positions.length]).toBeGreaterThan(8000);
      expect(arm.positions[local.positions.length + 2]).toBe(2 + elevation);
      expect(arm.indices.slice(3, 6)).toEqual(new Uint32Array([3, 4, 5]));
      expect(arm.extents.physical[2]).toBe(arm.positions.at(-6));
      previous = arm.positions.length;
    }
    expect(local.positions).toEqual(new Float32Array([-0, 0, 2, 20, 0, 3, 0, 20, 4]));
  },
);
