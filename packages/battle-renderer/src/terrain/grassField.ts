// The grass field's kinds on the page: the installed grass appearances, and
// their blade meshes handed to Rust to pack (`ground::grass::pack_shapes`).
// The biome's growth, the props' margins, the packed shapes and each frame's
// window and parameters are the ground's (`crates/presentation`'s
// `ground::grass`); where clumps stand, how many and how tall is decided per
// frame by the shared Rust renderer; nothing here is per clump.
//
// The field is a technique rewrite of ~/dev/game's
// grassField.ts and battleGrassResidency.ts, after the Ghost of Tsushima
// talk (research.md): the clumps are regrown on the GPU from world tiles
// whenever the view moves, so there is no CPU residency or upload, and a
// clump's place is a function of its tile alone, so what is drawn never
// reshuffles as the camera moves.
import type { StaticBundle } from "@packages/scene-assets/src/schema.ts";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader.ts";
import { GrassShapes } from "@wasm/renderer_browser";

/** Grass appearances (scenery kind "grass") by catalog name. */
export type GrassAppearances = ReadonlyMap<string, StaticBundle>;

/** The grass kinds in an installed catalog generation. */
export function grassAppearancesOf(installed: InstalledAppearances | null): GrassAppearances {
  const out = new Map<string, StaticBundle>();
  for (const [name, { scenery, bundle }] of installed?.appearances ?? [])
    if (scenery === "grass" && bundle.kind === "static") out.set(name, bundle);
  return out;
}

/** The kinds `names` (in the ground's kind order, `Ground.grass_kinds`)
 *  packed for the field from their bundles' first states. Throws, naming the
 *  kind, where a tier is not its blade strips. */
export function grassShapes(names: readonly string[], kinds: GrassAppearances): GrassShapes {
  const states = names.map((name) => kinds.get(name)!.states[0]);
  const tiers = states.flatMap((s) => s.tiers);
  const join = <T extends Float32Array | Int16Array | Uint8Array>(
    make: new (n: number) => T,
    of: (t: (typeof tiers)[number]) => T,
  ) => {
    const out = new make(tiers.reduce((n, t) => n + of(t).length, 0));
    let at = 0;
    for (const t of tiers) {
      out.set(of(t) as ArrayLike<number>, at);
      at += of(t).length;
    }
    return out;
  };
  return new GrassShapes(
    JSON.stringify(names),
    Uint32Array.from(states, (s) => s.tiers.length),
    Float64Array.from(states, (s) => s.bounds.max[2]),
    Uint32Array.from(tiers, (t) => t.positions.length / 3),
    join(Float32Array, (t) => t.positions),
    join(Int16Array, (t) => t.normals),
    join(Float32Array, (t) => t.uvs),
    join(Uint8Array, (t) => t.colors),
  );
}
