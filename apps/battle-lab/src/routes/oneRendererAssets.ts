import { readBundle } from "@packages/scene-assets/src/gzip";
import type { Bundle, RuntimeCatalog } from "@packages/scene-assets/src/schema";
import { posedPalette } from "@packages/battle-renderer/src/models/modelInstances";
import { FOUNDATION } from "./frame";
import type { ModelWorkload } from "./oneRendererModel";

let tankBundle: Promise<Exclude<Bundle, { kind: "clips" }>> | null = null;

export async function loadTankBundle(): Promise<Exclude<Bundle, { kind: "clips" }>> {
  if (!tankBundle) {
    tankBundle = (async () => {
      const catalog = (await (await fetch("/catalog.json")).json()) as RuntimeCatalog;
      const hash = catalog.appearances?.test_tank?.bundle;
      if (!hash) throw new Error("runtime catalog has no test_tank bundle");
      return (await readBundle(
        catalog,
        hash,
        async (path) => new Uint8Array(await (await fetch(`/${path}`)).arrayBuffer()),
      )) as Exclude<Bundle, { kind: "clips" }>;
    })();
  }
  return tankBundle;
}

export function mw(bundle: Exclude<Bundle, { kind: "clips" }>): ModelWorkload {
  const tier = 1;
  const posed = posedPalette(bundle, null);
  const parts =
    bundle.kind === "articulated"
      ? bundle.nodes.map((node, nodeIndex) => ({
          mesh: node.tiers[Math.min(tier, node.tiers.length - 1)],
          transform:
            posed.kind === "posed"
              ? posed.palette.subarray(nodeIndex * 12, nodeIndex * 12 + 12)
              : null,
        }))
      : [
          {
            mesh:
              bundle.kind === "skinned"
                ? bundle.tiers[Math.min(tier, bundle.tiers.length - 1)]
                : bundle.states[0].tiers[Math.min(tier, bundle.states[0].tiers.length - 1)],
            transform: null,
          },
        ];
  const vertices: number[] = [];
  const indices: number[] = [];
  for (const { mesh, transform } of parts) {
    if (!mesh) continue;
    const base = vertices.length / 9;
    for (let i = 0; i < mesh.positions.length / 3; i++) {
      let x = mesh.positions[i * 3];
      let y = mesh.positions[i * 3 + 1];
      let z = mesh.positions[i * 3 + 2];
      if (transform) {
        [x, y, z] = [
          transform[0] * x + transform[1] * y + transform[2] * z + transform[3],
          transform[4] * x + transform[5] * y + transform[6] * z + transform[7],
          transform[8] * x + transform[9] * y + transform[10] * z + transform[11],
        ];
      }
      vertices.push(
        x,
        y,
        z,
        mesh.colors[i * 4] / 255,
        mesh.colors[i * 4 + 1] / 255,
        mesh.colors[i * 4 + 2] / 255,
        mesh.colors[i * 4 + 3] / 255,
        mesh.uvs[i * 2],
        mesh.uvs[i * 2 + 1],
      );
    }
    for (const index of mesh.indices) indices.push(base + index);
  }
  const modelPlacements = FOUNDATION.filter((p) => p.kind === "test_tank").concat(
    FOUNDATION.filter((p) => p.kind === "test_tank").map((p) => ({
      ...p,
      x: p.x + 9,
      y: p.y + 7,
    })),
  );
  const instances = modelPlacements.flatMap((p, i) => [
    p.x,
    p.y,
    p.z,
    p.yaw,
    i % 2 ? 0.78 : 0.58,
    i % 2 ? 0.88 : 0.72,
    1.0,
    1.0,
  ]);
  const source = bundle.textures.find((texture) => texture.levels[0]?.length >= 4);
  const textureWidth = source?.width ?? 1;
  const textureHeight = source?.height ?? 1;
  return {
    vertices: new Float32Array(vertices),
    indices: new Uint32Array(indices),
    instances: new Float32Array(instances),
    texture: source?.levels[0] ?? Uint8Array.of(255, 255, 255, 255),
    textureWidth,
    textureHeight,
  };
}

/** A compact one-bundle model record stream for the shared renderer seam. */
export function fixtureModelFrame(instances: Float32Array) {
  const batches = new Uint32Array(12 * (instances.length / 16));
  for (let index = 0; index < instances.length / 16; index++) {
    const o = index * 12;
    batches[o] = index;
    batches[o + 2] = 1;
    batches[o + 3] = 0;
    batches[o + 4] = 1;
    batches[o + 6] = 1;
    batches[o + 7] = index;
    batches[o + 8] = 1;
    batches[o + 9] = 15;
  }
  return {
    palettes: new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0]),
    instances,
    batches,
  };
}

export function shadowBatchWords(batches: Uint32Array) {
  return new Uint32Array([...batches, 0, 0, 0, 1, 0, 0, 0, 0, 1, 15, 0, 0]);
}
