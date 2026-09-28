// The one appearance loader, for the workbench and the battle alike. It reads
// the runtime catalog, fetches every bundle by content hash, verifies each
// hash, decodes, and installs the whole set at once. A failure anywhere leaves
// the previously installed generation in place.

import type { Vec3 } from "math";
import { decodeBundle } from "./codec.ts";
import { lfsPointerOid, lfsPullCommand, sha256Hex } from "./glb.ts";
import {
  bundlePath,
  type Bundle,
  type RuntimeCatalog,
  type SideTints,
  type SkeletonClips,
  type AppearanceUnit,
} from "./schema.ts";
import type { MountDraws } from "./units.ts";

/** Where the runtime directory lives in the repo, for LFS pull hints. */
export const RUNTIME_DIR = "assets/runtime";

export interface InstalledAppearances {
  generation: number;
  sides: SideTints;
  skeletons: Map<string, SkeletonClips>;
  appearances: Map<
    string,
    {
      unit: AppearanceUnit;
      scenery: string | null;
      /** The simulation box a static appearance is authored to (catalog `footprint_half_m`). */
      footprint: Vec3 | null;
      /** A vehicle's rig per mount name (catalog `mounts`); null when it declares none. */
      mounts: MountDraws | null;
      bundle: Exclude<Bundle, SkeletonClips>;
    }
  >;
}

export type Fetch = (url: string) => Promise<{
  ok: boolean;
  status: number;
  arrayBuffer(): Promise<ArrayBuffer>;
  json(): Promise<unknown>;
}>;

export class AppearanceLibrary {
  private readonly fetcher: Fetch;
  private current: InstalledAppearances | null = null;

  constructor(fetcher: Fetch = (url) => fetch(url)) {
    this.fetcher = fetcher;
  }

  get installed(): InstalledAppearances | null {
    return this.current;
  }

  /** Load every catalog entry under `baseUrl` (ending in "/"); install only if all succeed. */
  async load(baseUrl: string): Promise<InstalledAppearances> {
    const response = await this.fetcher(`${baseUrl}catalog.json`);
    if (!response.ok) throw new Error(`appearance catalog: HTTP ${response.status}`);
    const catalog = (await response.json()) as RuntimeCatalog;
    const read = async (hash: string): Promise<Bundle> => {
      const url = `${baseUrl}${bundlePath(hash)}`;
      const res = await this.fetcher(url);
      if (!res.ok) throw new Error(`bundle ${hash}: HTTP ${res.status}`);
      const bytes = new Uint8Array(await res.arrayBuffer());
      if (lfsPointerOid(bytes) !== null)
        throw new Error(
          `bundle ${hash} is a Git LFS pointer; run: ${lfsPullCommand(`${RUNTIME_DIR}/${bundlePath(hash)}`)}`,
        );
      const actual = await sha256Hex(bytes);
      if (actual !== hash) throw new Error(`bundle ${hash}: content hash is ${actual}`);
      return decodeBundle(bytes);
    };
    const skeletons = new Map<string, SkeletonClips>();
    await Promise.all(
      Object.entries(catalog.skeletons ?? {}).map(async ([id, hash]) => {
        const bundle = await read(hash);
        if (bundle.kind !== "clips" || bundle.id !== id)
          throw new Error(`skeleton ${id}: bundle ${hash} is not its clips`);
        skeletons.set(id, bundle);
      }),
    );
    const appearances: InstalledAppearances["appearances"] = new Map();
    await Promise.all(
      Object.entries(catalog.appearances ?? {}).map(async ([name, entry]) => {
        const bundle = await read(entry.bundle);
        if (bundle.kind === "clips" || bundle.kind !== entry.kind)
          throw new Error(
            `appearance ${name}: bundle is ${bundle.kind}, catalog says ${entry.kind}`,
          );
        if (bundle.kind === "skinned") {
          const clips = skeletons.get(bundle.skeleton);
          if (!clips)
            throw new Error(
              `appearance ${name}: skeleton ${bundle.skeleton} is not in the catalog`,
            );
          const same =
            clips.joints.length === bundle.joints.length &&
            clips.joints.every(
              (j, i) => j.name === bundle.joints[i].name && j.parent === bundle.joints[i].parent,
            );
          if (!same)
            throw new Error(`appearance ${name}: joints differ from skeleton ${bundle.skeleton}`);
        }
        appearances.set(name, {
          unit: entry.unit,
          scenery: entry.scenery ?? null,
          footprint: entry.footprint_half_m ?? null,
          mounts: entry.mounts ?? null,
          bundle,
        });
      }),
    );
    if (!catalog.sides) throw new Error("appearance catalog has no side tints; re-bake");
    this.current = {
      generation: (this.current?.generation ?? 0) + 1,
      sides: catalog.sides,
      skeletons,
      appearances,
    };
    return this.current;
  }
}

/** A fetch over runtime files held in memory, keyed by their path under `baseUrl`. */
export function memoryFetch(files: ReadonlyMap<string, Uint8Array>, baseUrl: string): Fetch {
  return async (url) => {
    const bytes = url.startsWith(baseUrl) ? files.get(url.slice(baseUrl.length)) : undefined;
    return {
      ok: !!bytes,
      status: bytes ? 200 : 404,
      arrayBuffer: async () => bytes!.slice().buffer,
      json: async () => JSON.parse(new TextDecoder().decode(bytes)),
    };
  };
}
