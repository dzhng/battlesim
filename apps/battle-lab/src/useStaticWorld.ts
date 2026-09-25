import { useEffect, useState } from "react";
import type { WorldExports, WorldLayout } from "@packages/battle-renderer/src/worldMesh";
import { loadWasm } from "./wasm";

type Wasm = Awaited<ReturnType<typeof loadWasm>>;
export type WorldView = InstanceType<Wasm["WorldView"]>;

export interface StaticWorld {
  /** Main-thread queries over the public static map (picking, probes). */
  view: WorldView;
  layout: WorldLayout;
  exports: WorldExports;
}

/** The map every side knows from the start, built by the simulation's own
 *  geometry code from the same map JSON the authority uses. */
export function useStaticWorld(map: unknown): StaticWorld | null {
  const [world, setWorld] = useState<StaticWorld | null>(null);
  useEffect(() => {
    let live = true;
    let view: WorldView | null = null;
    void loadWasm().then((wasm) => {
      if (!live) return;
      view = new wasm.WorldView(JSON.stringify(map));
      setWorld({
        view,
        layout: JSON.parse(wasm.world_layout()) as WorldLayout,
        exports: {
          positions: view.terrain_positions(),
          indices: view.terrain_indices(),
          triangleSurfaces: view.terrain_triangle_surfaces(),
          props: view.props(),
          water: view.water(),
          forests: view.forests(),
        },
      });
    });
    return () => {
      live = false;
      view?.free();
    };
  }, [map]);
  return world;
}

/** Ground point under a camera ray on the authoritative surface, or null. */
export function groundUnderRay(
  view: WorldView,
  ray: { origin: readonly number[]; dir: readonly number[] },
): [number, number, number] | null {
  const hit = view.raycast(
    ray.origin[0],
    ray.origin[1],
    ray.origin[2],
    ray.dir[0],
    ray.dir[1],
    ray.dir[2],
    10_000,
  );
  return hit.length ? [hit[1], hit[2], hit[3]] : null;
}
