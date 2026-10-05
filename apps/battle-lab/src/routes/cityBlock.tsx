// /lab/city-block: a block of a generated town, drawn from the template art
// library, with no battle on it. The map is the generator's for a fixed
// request (?type=, ?size=, ?seed=, ?region= choose another), the block is
// the apartment building nearest the main town's centre that has a house
// beside it, and the camera stands at fixed stations round it: on the
// street, at the default tactical camera, 250 m out, low over the town from
// 2 km, and at the strategic overview. A switch makes the side see the block's apartment
// building destroyed, as the simulation would end it (collapsed, or gutted if
// it is tall); a probe replaces the map's buildings with every other one,
// as a new map would; `?tier=0..3` draws every building at one detail tier.
import { useCallback, useMemo, useRef, useState } from "react";
import { GAME_RULES } from "../scenarios";
import config from "@fixtures/generated-battle.json";
import presets from "@fixtures/map-presets.json?raw";
import templates from "@fixtures/prototype-building-templates.json?raw";
import {
  buildingObstacles,
  buildingPartProps,
} from "@packages/battle-renderer/src/buildingObstacles";
import {
  FRAME_FLOATS,
  type BuildingIndex,
  type PlacedBuildings,
} from "@packages/battle-renderer/src/models/buildingReferences";
import { PropAppearances } from "@packages/battle-renderer/src/models/propAppearance";
import {
  apartKinds,
  buildWorldLayers,
  type PublicBuildings,
} from "@packages/battle-renderer/src/worldMesh";
import { CameraController, type CameraPose } from "@packages/renderer-core/src/cameraController";
import {
  canonicalSeed,
  generationRequest,
  MAP_SIZES,
  MAP_TYPES,
  resolveMap,
  type MapChoice,
} from "@web/maps/source";
import { knownFallen, seenDestroyed } from "../destroyedBuildings";
import { useFeed } from "../feed";
import { mapAppearances, useMapAppearances } from "../gameAppearances";
import { gameBiome } from "../gameBiome";
import { gameCamera } from "../gameCamera";
import { LabViewport, type ViewportPilot } from "../LabViewport";
import { askedTier, tierBoundaries, useBuildingTier } from "../buildingTier";
import { buildFailed, useBuiltScenario } from "../useBuiltScenario";
import { useMapBuildings, useStaticWorld, type StaticWorld } from "../useStaticWorld";

// The same explicit rule record a battle carries, including forest geometry.
const GENERATOR = { presets, templates, rules: JSON.stringify(GAME_RULES) };
/** The map the lab opens on: a town with houses and apartment blocks. */
const DEFAULT: MapChoice = { type: "mixed", size: "medium", seed: "1" };
/** A house this near an apartment building makes the two a block, metres. */
const BLOCK_REACH_M = 90;
/** The third station's distance, metres. */
const WIDE_M = 250;
/** The oblique station: the town from far off and low, most of the map
 *  behind it up to the horizon. A player's orbit goes lower still; this is a
 *  view a player holds. */
const OBLIQUE = { distance_m: 2000, pitch: 0.3 };
/** How far along a street the street station looks for its run, metres. */
const STREET_RUN_M = 80;

type Category = PublicBuildings["buildings"][number]["category"];
const APARTMENTS: readonly Category[] = ["urban_apartment"];
const HOUSES: readonly Category[] = ["detached_home", "attached_home"];

/** The map a query asks for, or the lab's own. */
function askedMap(search: string): MapChoice {
  const query = new URLSearchParams(search);
  const type = MAP_TYPES.find((t) => t === query.get("type"));
  const size = MAP_SIZES.find((s) => s === query.get("size"));
  const seed = canonicalSeed(query.get("seed") ?? "");
  const region = query.get("region");
  return {
    type: type ?? DEFAULT.type,
    size: size ?? DEFAULT.size,
    seed: seed ?? DEFAULT.seed,
    ...(region !== null && { region }),
  };
}

interface GeneratedTown {
  map: unknown;
  size: [number, number];
  /** Where the main settlement's streets meet. */
  town: [number, number];
}

/** The block: the art-drawn apartment building nearest `town` with an
 *  art-drawn house within reach, the two buildings and the point between
 *  them; the building nearest `town` alone where the map has no such pair. */
function blockOf(buildings: PublicBuildings, index: BuildingIndex, town: [number, number]) {
  const owners = new Map(Array.from(index.placed.owners, (owner, i) => [owner, i]));
  const drawn = buildings.buildings.flatMap((b) => {
    const at = owners.get(b.owner);
    return at === undefined ? [] : [{ at, category: b.category, xy: b.frame.translation }];
  });
  const apart = (a: readonly number[], b: readonly number[]) =>
    Math.hypot(a[0] - b[0], a[1] - b[1]);
  const nearest = <T extends { xy: readonly number[] }>(list: T[], to: readonly number[]) =>
    list.reduce<T | null>((a, b) => (a && apart(a.xy, to) <= apart(b.xy, to) ? a : b), null);
  const houses = drawn.filter((b) => HOUSES.includes(b.category));
  const pairs = drawn
    .filter((b) => APARTMENTS.includes(b.category))
    .flatMap((apartment) => {
      const house = nearest(houses, apartment.xy);
      return house && apart(house.xy, apartment.xy) <= BLOCK_REACH_M ? [{ apartment, house }] : [];
    })
    .map((pair) => ({ ...pair, xy: pair.apartment.xy }));
  const pair = nearest(pairs, town);
  if (pair)
    return {
      apartment: pair.apartment.at,
      house: pair.house.at,
      center: [
        (pair.apartment.xy[0] + pair.house.xy[0]) / 2,
        (pair.apartment.xy[1] + pair.house.xy[1]) / 2,
      ] as [number, number],
    };
  const only = nearest(drawn, town);
  return {
    apartment: only?.at ?? null,
    house: null,
    center: (only ? [only.xy[0], only.xy[1]] : town) as [number, number],
  };
}

/** The street nearest `center`: a point on a road, and the heading of the
 *  longest straight run of road through it. Null where no road is near. */
function streetNear(world: StaticWorld, center: [number, number]) {
  const road = world.layout.surfaceKinds.indexOf("road");
  const onRoad = (x: number, y: number) => world.view.surface_at(x, y)[5] === road;
  const run = (x: number, y: number, heading: number) => {
    let m = 0;
    while (
      m < STREET_RUN_M &&
      onRoad(x + Math.cos(heading) * (m + 2), y + Math.sin(heading) * (m + 2))
    )
      m += 2;
    return m;
  };
  for (let r = 4; r <= BLOCK_REACH_M; r += 4)
    for (let k = 0; k < 16; k++) {
      const [x, y] = [
        center[0] + r * Math.cos((k * Math.PI) / 8),
        center[1] + r * Math.sin((k * Math.PI) / 8),
      ];
      if (!onRoad(x, y)) continue;
      let best = { heading: 0, m: -1 };
      for (let h = 0; h < 32; h++) {
        const heading = (h * Math.PI) / 16;
        // Looking along `heading`, with the eye back down the street behind.
        const m = run(x, y, heading) + Math.min(run(x, y, heading + Math.PI), 24);
        if (m > best.m) best = { heading, m };
      }
      return { at: [x, y] as [number, number], heading: best.heading };
    }
  return null;
}

/** Every other building of `placed`: another map's worth of references, as
 *  the frame sees it. */
function everyOther(placed: PlacedBuildings): PlacedBuildings {
  const kept = Array.from(placed.template.keys()).filter((i) => i % 2 === 0);
  return {
    templates: placed.templates,
    template: Uint16Array.from(kept, (i) => placed.template[i]),
    frames: Float64Array.from(
      kept.flatMap((i) => [...placed.frames.subarray(i * FRAME_FLOATS, (i + 1) * FRAME_FLOATS)]),
    ),
    owners: Uint32Array.from(kept, (i) => placed.owners[i]),
  };
}

export default function CityBlock() {
  const [choice] = useState(() => askedMap(window.location.search));
  const generated = useBuiltScenario(choice, async (wasm, asked): Promise<GeneratedTown> => {
    const request = generationRequest(wasm, asked, GENERATOR, config.limits);
    const map = await resolveMap(
      { kind: "generated", request },
      {
        generator: wasm,
        documents: GENERATOR,
        loadMap: () => {
          throw new Error("the city block lab draws generated maps only");
        },
      },
    );
    const sites = JSON.parse(map.sites!) as { settlements: { center: [number, number] }[] };
    const { size } = map.definition as { size: [number, number] };
    return { map: map.definition, size, town: sites.settlements[0]?.center ?? [0, 0] };
  });
  if (!generated) return null;
  if (buildFailed(generated))
    return (
      <main style={{ padding: 24 }} className="lab-rejected" data-testid="error">
        the city block's map could not be generated: {generated.error}
      </main>
    );
  return <Block choice={choice} generated={generated} />;
}

function Block({ choice, generated }: { choice: MapChoice; generated: GeneratedTown }) {
  const world = useStaticWorld(generated.map, GAME_RULES);
  // The map's buildings: every one a template reference.
  const drawn = useMapBuildings(world);
  // The catalog, and the kits this town's buildings draw from.
  const appearances = useMapAppearances(
    drawn?.index.placed ?? null,
    world?.exports.buildings.regionalFamily ?? null,
  );
  const surfaceZ = useCallback(
    (x: number, y: number) => world?.view.surface_at(x, y)[0] ?? 0,
    [world],
  );
  const meshes = useMemo(
    () =>
      world &&
      appearances &&
      buildWorldLayers(
        world.exports,
        world.layout,
        gameBiome,
        "surface",
        apartKinds(world.layout, true),
        appearances,
      ),
    [world, appearances],
  );
  const worldFeed = useFeed(meshes);
  const modelAppearances = useMemo(
    () =>
      world && appearances && drawn
        ? mapAppearances(
            appearances,
            drawn.props,
            new PropAppearances(appearances, world.layout, world.exports.buildings.regionalFamily),
            drawn.index.placed,
            false,
          )
        : null,
    [world, appearances, drawn],
  );
  const block = useMemo(
    () => world && drawn && blockOf(world.exports.buildings, drawn.index, generated.town),
    [world, drawn, generated],
  );

  // What the side knows: the map, or that it has seen the block's apartment
  // building destroyed.
  const [fallen, setFallen] = useState(false);
  const [halved, setHalved] = useState(false);
  const half = useMemo(() => drawn && everyOther(drawn.index.placed), [drawn]);
  const standing = useMemo(() => {
    const library = appearances?.templates?.library;
    if (!world || !drawn || !block || !half || !library) return null;
    const { props, index } = drawn;
    const known =
      fallen && !halved && block.apartment !== null
        ? seenDestroyed(index, block.apartment, library)
        : [];
    return {
      buildings: {
        placed: halved ? half : index.placed,
        fallen: knownFallen(index, known),
      },
      obstacles: buildingObstacles(
        props,
        known,
        buildingPartProps(world.exports.buildings),
        surfaceZ,
      ),
    };
  }, [world, drawn, block, half, fallen, halved, surfaceZ, appearances]);
  const buildingsFeed = useFeed(standing?.buildings ?? null);
  const obstaclesFeed = useFeed(standing?.obstacles ?? null);
  const { style, drawAt } = useBuildingTier(askedTier(window.location.search));

  // The stations, each a framing the rig places as a scripted camera's.
  const cameraConfig = useMemo(() => gameCamera.forMap(generated.size), [generated]);
  const rig = useMemo(() => new CameraController(cameraConfig), [cameraConfig]);
  /** The rig's framing of `target` from `distance`. */
  const at = useCallback(
    (target: [number, number], distance: number, yaw: number): CameraPose => ({
      target,
      distance,
      yaw,
      pitch: rig.pitchAt(distance),
    }),
    [rig],
  );
  const stations = useMemo(() => {
    if (!world || !block) return null;
    const opening = gameCamera.opening();
    const street = streetNear(world, block.center);
    return {
      // Standing in the street, looking along it.
      street: street
        ? at(street.at, cameraConfig.zoom_min, street.heading + Math.PI)
        : at(block.center, cameraConfig.zoom_min, opening.yaw),
      tactical: at(block.center, opening.distance, opening.yaw),
      wide: at(block.center, WIDE_M, opening.yaw),
      oblique: { ...at(block.center, OBLIQUE.distance_m, opening.yaw), pitch: OBLIQUE.pitch },
      overview: at(
        [generated.size[0] / 2, generated.size[1] / 2],
        cameraConfig.zoom_max,
        opening.yaw,
      ),
    };
  }, [world, block, generated, cameraConfig, at]);
  type Station = keyof NonNullable<typeof stations>;
  const [station, setStation] = useState<Station>("tactical");
  // Where the camera is cut to, once, when a station is chosen.
  const cut = useRef<CameraPose | null>(null);
  const [pilot] = useState<ViewportPilot>(() => ({
    pose() {
      const once = cut.current;
      cut.current = null;
      return once;
    },
  }));
  const stand = useCallback(
    (next: Station) => {
      if (!stations) return;
      cut.current = stations[next];
      setStation(next);
    },
    [stations],
  );

  const diagnostics = useMemo(
    () =>
      world &&
      drawn &&
      block &&
      stations && {
        map: () => ({ ...choice, size: generated.size, town: generated.town }),
        stations: () => stations,
        /** Cut the camera to a station, as the rig places it. */
        stand,
        /** The rig's framing of the block from `distance` metres. */
        poseAt: (distance: number) => at(block.center, distance, gameCamera.opening().yaw),
        /** How far off each tier boundary is in this window, metres. */
        boundaries: tierBoundaries,
        /** Every building at one tier (null: by distance); resolves once drawn. */
        drawAt,
        block: () => block,
        surfaceZ,
        surfaceAt: (x: number, y: number) => {
          const s = world.view.surface_at(x, y);
          return s.length ? { kind: world.layout.surfaceKinds[s[5]], forest: s[6] === 1 } : null;
        },
        /** The map's buildings: each one's template, category, frame and its
         *  parts' boxes on the map. */
        buildings: () => {
          const { placed } = drawn.index;
          const category = new Map(
            world.exports.buildings.buildings.map((b) => [b.owner, b.category]),
          );
          return drawn.index.parts.map((parts, i) => ({
            template: placed.templates[placed.template[i]],
            category: category.get(placed.owners[i]),
            frame: [...placed.frames.subarray(i * FRAME_FLOATS, (i + 1) * FRAME_FLOATS)],
            parts: parts.map((p) => ({
              center: p.center,
              baseZ: p.baseZ,
              yaw: p.yaw,
              half: p.half,
            })),
          }));
        },
        /** How many buildings the map holds. */
        counts: () => ({ buildings: world.exports.buildings.buildings.length }),
        setFallen,
        /** Draw every other building only (another set of references, as a
         *  new map's would be), or all of them again. */
        setHalved,
      },
    [world, drawn, block, stations, stand, at, drawAt, surfaceZ, choice, generated],
  );

  if (!world || !meshes || !stations) return null;
  const opening = stations.tactical;
  return (
    <>
      <LabViewport
        fixture="city-block"
        world={worldFeed}
        buildings={buildingsFeed}
        buildingStyle={style}
        obstacles={obstaclesFeed}
        appearances={modelAppearances}
        initialCamera={{
          ...gameCamera.opening(),
          target: [...opening.target, surfaceZ(...opening.target)],
        }}
        groundAt={surfaceZ}
        pilot={pilot}
        cameraConfig={cameraConfig}
        diagnostics={diagnostics || undefined}
      />
      <aside className="hud-panel lab-panel" data-testid="city-block-panel">
        <strong>City block</strong>
        <div className="lab-hint">
          {choice.type} · {choice.size} · seed {choice.seed}
          {choice.region && ` · ${choice.region}`}. WASD/arrows pan · Q/E turn · middle‑drag orbit ·
          wheel zoom
        </div>
        <div className="lab-row">
          {(Object.keys(stations) as Station[]).map((id) => (
            <button key={id} type="button" aria-pressed={id === station} onClick={() => stand(id)}>
              {id}
            </button>
          ))}
        </div>
        <label>
          <input type="checkbox" checked={fallen} onChange={(e) => setFallen(e.target.checked)} />
          The side has seen the apartment building destroyed
        </label>
      </aside>
    </>
  );
}
