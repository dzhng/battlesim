/** Generated primitives are ordinary canonical posed model assets. */
import { PROXY_MESHES, type DrawnProxyKind } from "@packages/battle-renderer/src/proxies";
import type { SceneInstance } from "@packages/battle-renderer/src/scene";
import { positionsBounds } from "@packages/scene-assets/src/pose";
import type { ArticulatedBundle } from "@packages/scene-assets/src/schema";
import type { LabModel } from "../sim/presentationModels";
import { generatedContentId } from "./contentIdentity";
import { modelAsset, type ModelAsset } from "./modelAssets";

const PREFIX = "__renderer_proxy__:";

/** The one battle appearance Rust uses while a unit's authored art is absent. */
export const BATTLE_STAND_IN_APPEARANCE = "__battle_stand_in__";

/** Whether an admitted appearance is a generated primitive. */
export function isProxyAppearance(name: string): boolean {
  return name.startsWith(PREFIX);
}

function proxyBundle(kind: DrawnProxyKind, tint: readonly number[]): ArticulatedBundle {
  const source = PROXY_MESHES[kind];
  const count = source.length / 10;
  const positions = new Float32Array(count * 3);
  const normals = new Int16Array(count * 4);
  const colors = new Uint8Array(count * 4).fill(255);
  for (let vertex = 0; vertex < count; vertex++) {
    positions.set(source.subarray(vertex * 10, vertex * 10 + 3), vertex * 3);
    for (let c = 0; c < 3; c++)
      normals[vertex * 4 + c] = Math.round(source[vertex * 10 + 3 + c] * 32767);
  }
  const linear = (c: number) =>
    Math.fround(
      Math.pow(Math.max(0, Math.fround(source[6 + c] * Math.fround(tint[c]))), Math.fround(2.2)),
    );
  const mesh = {
    positions,
    normals,
    colors,
    uvs: new Float32Array(count * 2),
    indices: Uint16Array.from({ length: count }, (_, i) => i),
    draws: [{ material: 0, first: 0, count }],
  };
  return {
    kind: "articulated",
    nodes: [
      {
        name: "body",
        parent: -1,
        pivot: [0, 0, 0],
        bind: { t: [0, 0, 0], r: [0, 0, 0, 1], s: [1, 1, 1] },
        extras: {},
        tiers: [mesh, mesh, mesh, mesh],
      },
    ],
    materials: [
      {
        name: "proxy",
        base_color: [linear(0), linear(1), linear(2), 1],
        metallic: 0,
        roughness: 0.85,
        tint: 0,
        coverage: { kind: "opaque" },
      },
    ],
    textures: [],
    bounds: positionsBounds(positions),
  };
}

const battleStandInBundle = proxyBundle("box", [1, 1, 1]);
const battleStandInBounds = battleStandInBundle.bounds;
export const BATTLE_STAND_IN_HALF_EXTENTS = [
  (battleStandInBounds.max[0] - battleStandInBounds.min[0]) / 2,
  (battleStandInBounds.max[1] - battleStandInBounds.min[1]) / 2,
  (battleStandInBounds.max[2] - battleStandInBounds.min[2]) / 2,
] as const;

/** The battle stand-in is admitted with the first generation, before any
 * withheld unit art can arrive. It is an ordinary articulated box asset. */
const BATTLE_STAND_IN_ASSET = modelAsset(
  BATTLE_STAND_IN_APPEARANCE,
  battleStandInBundle,
  generatedContentId(battleStandInBundle),
);

export function battleStandInAsset(): ModelAsset {
  return BATTLE_STAND_IN_ASSET;
}

/** Active-generation deduplication only; transforms and highlights do not own art. */
export function proxyModels(
  instances: readonly SceneInstance[],
  occupied: Iterable<string> = [],
): { assets: ModelAsset[]; models: LabModel[] } {
  const installed = new Set(occupied);
  const assets = new Map<string, ModelAsset>();
  const models: LabModel[] = [];
  for (const instance of instances) {
    if (instance.kind === "infantry") continue;
    const bits = new Uint32Array(Float32Array.from(instance.color).buffer);
    const name = `${PREFIX}${instance.kind}:${Array.from(bits, (v) => v.toString(16)).join(":")}`;
    if (installed.has(name))
      throw new Error(`proxy appearance ${name} collides with installed art`);
    if (!assets.has(name)) {
      const bundle = proxyBundle(instance.kind, instance.color);
      assets.set(name, modelAsset(name, bundle, generatedContentId(bundle)));
    }
    models.push({
      appearance: name,
      x: instance.x,
      y: instance.y,
      z: instance.z,
      yaw: instance.yaw,
      pose: { kind: "rest" },
      tier: 0,
      highlight: instance.highlight,
    });
  }
  return { assets: [...assets.values()], models };
}
