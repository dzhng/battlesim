import { prepareStaticWorld } from "@web/battle/prepare/client";
import type { PublicWorldData } from "@web/battle/sim/publicWorld";
import { useEffect, useMemo, useState } from "react";
import type { WorldRay } from "@packages/renderer-core/src/camera3d";
import {
  indexBuildings,
  type SideBuildings,
} from "@packages/battle-renderer/src/models/buildingReferences";
import { mapProps } from "@packages/battle-renderer/src/models/propAppearance";
import {
  readWorldExports,
  type WorldExports,
  type WorldLayout,
} from "@packages/battle-renderer/src/worldMesh";
import { loadWasm, type Wasm } from "@web/battle/sim/module";
import { useFeed, type Feed } from "./feed";
import { GAME_RULES } from "./scenarios";

export type WorldView = InstanceType<Wasm["WorldView"]>;

export interface StaticWorld {
  /** Main-thread queries over the public static map (picking, probes). */
  view: Pick<WorldView, "height_at" | "surface_at" | "raycast" | "foliage_cleared">;
  layout: WorldLayout;
  exports: WorldExports;
}

/** The map every side knows from the start, built by the simulation's own
 *  geometry code from the same map JSON the authority uses. */
export function useWorldProbe(map: unknown): (StaticWorld & { view: WorldView }) | null {
  const [world, setWorld] = useState<(StaticWorld & { view: WorldView }) | null>(null);
  useEffect(() => {
    let live = true;
    let view: WorldView | null = null;
    void loadWasm().then((wasm) => {
      if (!live) return;
      // The world as the simulation builds it under the one rules owner.
      const rules = JSON.stringify(GAME_RULES);
      view = new wasm.WorldView(JSON.stringify(map), rules);
      setWorld({
        view,
        layout: JSON.parse(wasm.world_layout(rules)) as WorldLayout,
        exports: readWorldExports(view),
      });
    });
    return () => {
      live = false;
      view?.free();
    };
  }, [map]);
  return world;
}

/** The page holds query data, never the simulation's terrain/navigation world. */
export function useStaticWorld(
  data: Promise<PublicWorldData> | null,
  onError?: (message: string) => void,
): StaticWorld | null {
  const [built, setBuilt] = useState<{ data: Promise<PublicWorldData>; world: StaticWorld } | null>(
    null,
  );
  useEffect(() => {
    let live = true;
    let view: InstanceType<Wasm["PublicWorld"]> | null = null;
    setBuilt(null);
    if (data)
      void Promise.all([loadWasm(), data])
        .then(([wasm, publicWorld]) => {
          if (!live) return;
          view = new wasm.PublicWorld(publicWorld.queries);
          setBuilt({
            data,
            world: { view, layout: publicWorld.layout, exports: publicWorld.exports },
          });
        })
        .catch((error: unknown) => {
          if (live) {
            if (onError) onError(error instanceof Error ? error.message : String(error));
            else console.error("Public world failed to load", error);
          }
        });
    return () => {
      live = false;
      view?.free();
    };
  }, [data, onError]);
  return built?.data === data ? built.world : null;
}

/** A non-battle lab acquires only its public query/export data from a worker. */
export function useStaticMap(map: string): StaticWorld | null {
  const [data, setData] = useState<Promise<PublicWorldData> | null>(null);
  useEffect(() => {
    const preparation = prepareStaticWorld(map, JSON.stringify(GAME_RULES));
    setData(preparation.world);
    return () => preparation.cancel();
  }, [map]);
  return useStaticWorld(data);
}

/** Ground point under a camera ray on the authoritative surface, or null. */
export function groundUnderRay(
  view: Pick<WorldView, "raycast">,
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

/** The map's buildings for a view no side's knowledge is behind (a probe of
 *  the static world): every one standing. */
export function useStandingBuildings(world: StaticWorld | null): Feed<SideBuildings | null> {
  const buildings = useMemo<SideBuildings | null>(
    () =>
      world && {
        placed: indexBuildings(world.exports.buildings, mapProps(world.exports, world.layout))
          .placed,
        fallen: [],
      },
    [world],
  );
  return useFeed(buildings);
}
