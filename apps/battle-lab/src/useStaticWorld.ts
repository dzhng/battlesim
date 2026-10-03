import { useEffect, useMemo, useState } from "react";
import type { WorldRay } from "@packages/renderer-core/src/camera3d";
import {
  indexBuildings,
  type BuildingIndex,
  type SideBuildings,
} from "@packages/battle-renderer/src/models/buildingReferences";
import { mapProps, type MapProp } from "@packages/battle-renderer/src/models/propAppearance";
import {
  readWorldExports,
  type WorldExports,
  type WorldLayout,
} from "@packages/battle-renderer/src/worldMesh";
import { loadWasm, type Wasm } from "@web/battle/sim/module";

export type WorldView = InstanceType<Wasm["WorldView"]>;

export interface StaticWorld {
  /** Main-thread queries over the public static map (picking, probes). */
  view: WorldView;
  layout: WorldLayout;
  exports: WorldExports;
}

/** The map every side knows from the start, built on the page by the
 *  simulation's own geometry code from the same map the authority uses: its
 *  meshes, and the one implementation of picking, ground height, surface,
 *  learned foliage and camera clearance. It has no navigation and no battle
 *  state; the battle's world stays in its worker. */
export function useStaticWorld(map: unknown, rules: unknown): StaticWorld | null {
  const [world, setWorld] = useState<StaticWorld | null>(null);
  useEffect(() => {
    let live = true;
    let view: WorldView | null = null;
    void loadWasm().then((wasm) => {
      if (!live) return;
      // The world as the simulation builds it under the one rules owner.
      const ruleText = JSON.stringify(rules);
      view = new wasm.WorldView(JSON.stringify(map), ruleText);
      setWorld({
        view,
        layout: JSON.parse(wasm.world_layout(ruleText)) as WorldLayout,
        exports: readWorldExports(view),
      });
    });
    return () => {
      live = false;
      view?.free();
    };
  }, [map, rules]);
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
  return building?.owner ?? null;
}

/** The static map's props, and its buildings as references to their
 *  templates with the props that are their parts. */
export interface MapBuildings {
  props: MapProp[];
  index: BuildingIndex;
}

export function useMapBuildings(world: StaticWorld | null): MapBuildings | null {
  return useMemo(() => {
    if (!world) return null;
    const props = mapProps(world.exports, world.layout);
    return { props, index: indexBuildings(world.exports.buildings, props) };
  }, [world]);
}

/** The map's buildings for a view no side's knowledge is behind (a probe of
 *  the static world): every one standing. */
export function useStandingBuildings(world: StaticWorld | null): SideBuildings | null {
  const map = useMapBuildings(world);
  return useMemo(() => map && { placed: map.index.placed, fallen: [] }, [map]);
}
