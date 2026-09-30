import { useEffect, useState } from "react";
import type { WorldRay } from "@packages/renderer-core/src/camera3d";
import type { WorldExports, WorldLayout } from "@packages/battle-renderer/src/worldMesh";
import { loadWasm, type Wasm } from "@web/battle/sim/module";
import { VILLAGE_RULES } from "./scenarios";

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
      // The world as the simulation builds it under the one rules owner.
      const rules = JSON.stringify(VILLAGE_RULES);
      view = new wasm.WorldView(JSON.stringify(map), rules);
      setWorld({
        view,
        layout: JSON.parse(wasm.world_layout(rules)) as WorldLayout,
        exports: {
          terrain: {
            ...JSON.parse(view.terrain_grid()),
            pageIds: view.terrain_page_ids(),
            heights: view.terrain_heights(),
          },
          positions: view.terrain_positions(),
          indices: view.terrain_indices(),
          triangleSurfaces: view.terrain_triangle_surfaces(),
          props: view.props(),
          buildings: JSON.parse(view.buildings()),
          water: view.water(),
          forests: view.forests(),
          foliage: view.foliage(),
          roads: view.roads(),
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

/** The static building under a camera ray, if the first thing it meets is one. */
export function buildingUnderRay(world: StaticWorld, ray: WorldRay): number | null {
  const hit = world.view.raycast(...ray.origin, ...ray.dir, 10_000);
  if (!hit.length || hit[7] < 0) return null;
  const building = world.exports.buildings.buildings.find((b) =>
    b.parts.some((p) => p.prop === hit[7]),
  );
  return building && world.layout.garrisonPropKinds.includes(building.kind) ? building.owner : null;
}
