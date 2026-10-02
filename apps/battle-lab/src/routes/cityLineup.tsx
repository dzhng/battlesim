// /lab/city-lineup: every building template of the installed art library on
// flat ground, in rows by category at its real size, each labelled with its
// id and with no battle on it. The picture every art and damage pass on the
// templates is judged by.
//
// The buildings are a list of references handed to the frame as a generated
// town's are (`BattleFrame.setBuildings`), so what is drawn here is what a
// town draws. The address chooses what stands and how it is drawn:
// - `?set=` or `?category=`: one source set or category;
// - `?tier=0..3`: every building at that detail tier (by distance without it);
// - `?state=ruin|gutted`: every building as a side that saw it fall knows it;
// - `?station=` and `?at=<template id>`: where the camera opens. `all` and
//   `row` frame the line-up and the template's row; `fit` frames the
//   template; `close`, `tactical` and `wide` orbit it at 30 m, the game's
//   opening distance and 250 m, pitched as the game's camera is there; and
//   `transition-1..3` stand where it changes to that tier, so the tier before
//   and the tier after can be drawn from one pose;
// - `?labels=0`: no labels.
import { useCallback, useMemo, useRef, useState } from "react";
import catalogue from "@fixtures/prototype-building-templates.json";
import type { SideBuildings } from "@packages/battle-renderer/src/models/buildingReferences";
import { apartKinds, buildWorldLayers } from "@packages/battle-renderer/src/worldMesh";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { CameraController, type CameraPose } from "@packages/renderer-core/src/cameraController";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import { TIER_COUNT } from "@packages/scene-assets/src/schema";
import {
  TEMPLATE_STATES,
  type RowRange,
  type TemplateState,
} from "@packages/scene-assets/src/templateLibrary";
import type { Project } from "@web/battle/present/readouts";
import {
  cameraToFit,
  lineUp,
  lineupBuildings,
  lineupFallen,
  poseAtRange,
  type Lineup,
  type LineupEntry,
  type LineupTemplate,
} from "../cityLineup";
import { useFeed } from "../feed";
import { useGameAppearances } from "../gameAppearances";
import { gameBiome } from "../gameBiome";
import { gameCamera } from "../gameCamera";
import { LabViewport } from "../LabViewport";
import { askedTier, tierBoundaries, useBuildingTier } from "../buildingTier";
import { useStaticWorld } from "../useStaticWorld";

const SPACING = { gap_m: 14, row_gap_m: 24, margin_m: 48, backdrop: 2.5, grid_m: 64 };
/** The close station's orbit distance, off the foot of the building's front. */
const CLOSE_M = 30;
/** The wide station's orbit distance. */
const WIDE_M = 250;
/** How much of the picture's half width and height a framed row or template
 *  may reach to. */
const FILL = 0.88;
/** How far in front of a building its label is anchored, metres. */
const LABEL_FRONT_M = 3;
/** Every other label of a row sits this much lower, pixels: an id is often
 *  wider than its building. */
const LABEL_STAGGER_PX = 17;

/** The stations a single template is seen from, and those that frame many. */
const TEMPLATE_STATIONS = [
  "fit",
  "close",
  "tactical",
  "wide",
  "transition-1",
  "transition-2",
  "transition-3",
] as const;
type TemplateStation = (typeof TEMPLATE_STATIONS)[number];
type Station = TemplateStation | "row" | "all";

interface Asked {
  set: string | null;
  category: string | null;
  tier: number | null;
  state: TemplateState;
  station: Station;
  at: string | null;
  labels: boolean;
}

function asked(search: string): Asked {
  const query = new URLSearchParams(search);
  const station = query.get("station");
  return {
    set: query.get("set"),
    category: query.get("category"),
    tier: askedTier(search),
    state: TEMPLATE_STATES.find((s) => s === query.get("state")) ?? "intact",
    station:
      station === "row" || TEMPLATE_STATIONS.some((s) => s === station)
        ? (station as Station)
        : "all",
    at: query.get("at"),
    labels: query.get("labels") !== "0",
  };
}

/** A flat, empty map `size` metres across. */
const flatMap = (size: [number, number]) => ({
  size,
  fog_cell_m: 8,
  height_grid_m: 4,
  slope_cutoff_deg: 35,
  relief: [],
  surfaces: [],
  bridges: [],
  forests: [],
  props: [],
  buildings: [],
});

/** What a template's rows draw in each state the library has: per tier, the
 *  module instances and their triangles. */
function templateCosts(
  installed: NonNullable<InstalledAppearances["templates"]>,
  kits: InstalledAppearances,
) {
  const { library, modules } = installed;
  const triangles = modules.map((m) => {
    const bundle = kits.appearances.get(m.kit)?.bundle;
    const tiers = bundle?.kind === "static" ? bundle.states[m.state]?.tiers : undefined;
    return Array.from({ length: TIER_COUNT }, (_, t) => (tiers?.[t]?.indices.length ?? 0) / 3);
  });
  const cost = (range: RowRange) => {
    const rows = Array.from({ length: TIER_COUNT }, () => 0);
    const drawn = Array.from({ length: TIER_COUNT }, () => 0);
    for (let r = range.first; r < range.first + range.count; r++)
      for (let t = 0; t < TIER_COUNT; t++)
        if (library.rows.tiers[r] & (1 << t)) {
          rows[t]++;
          drawn[t] += triangles[library.rows.module[r]][t];
        }
    return { rows, triangles: drawn };
  };
  return library.templates.map((t) => ({
    id: t.id,
    set: t.set,
    status: t.status,
    states: Object.fromEntries(
      Object.entries(t.states).map(([state, range]) => [state, cost(range)]),
    ),
  }));
}

export default function CityLineup() {
  const [query] = useState(() => asked(window.location.search));
  const appearances = useGameAppearances();
  // The catalogue's templates the library dresses, as the address narrows them.
  const standing = useMemo(() => {
    if (!appearances) return null;
    const art = new Map(appearances.templates?.library.templates.map((t) => [t.id, t]));
    const all = (catalogue as unknown as Omit<LineupTemplate, "set">[]).map((t) => ({
      id: t.id,
      category: t.category,
      parts: t.parts,
      set: art.get(t.id)?.set ?? null,
    }));
    const dressed = all.filter((t): t is LineupTemplate => t.set !== null);
    return {
      /** Catalogue templates the library has no art for. */
      missing: all.filter((t) => t.set === null).map((t) => t.id),
      lineup: lineUp(
        dressed.filter(
          (t) =>
            (query.set === null || t.set === query.set) &&
            (query.category === null || t.category === query.category),
        ),
        SPACING,
      ),
    };
  }, [appearances, query]);
  if (!appearances || !standing) return null;
  if (!standing.lineup.entries.length)
    return (
      <main style={{ padding: 24 }} className="lab-rejected" data-testid="error">
        no template of the installed library matches this address
        {standing.missing.length > 0 && ` (without art: ${standing.missing.join(", ")})`}
      </main>
    );
  return (
    <Rows
      query={query}
      appearances={appearances}
      lineup={standing.lineup}
      missing={standing.missing}
    />
  );
}

function Rows({
  query,
  appearances,
  lineup,
  missing,
}: {
  query: Asked;
  appearances: InstalledAppearances;
  lineup: Lineup;
  missing: string[];
}) {
  const world = useStaticWorld(useMemo(() => flatMap(lineup.size), [lineup]));
  const surfaceZ = useCallback(
    (x: number, y: number) => world?.view.surface_at(x, y)[0] ?? 0,
    [world],
  );
  const meshes = useMemo(
    () =>
      world &&
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
  // The models layer installs the kits and the library, nothing else.
  const kits = useMemo<InstalledAppearances>(
    () => ({
      ...appearances,
      appearances: new Map([...appearances.appearances].filter(([, a]) => a.unit === "kit")),
    }),
    [appearances],
  );

  // What stands: the whole line-up or one template alone, in one state.
  const [solo, setSolo] = useState<string | null>(null);
  const [state, setState] = useState<TemplateState>(query.state);
  const buildings = useMemo<SideBuildings>(() => {
    const shown = lineup.entries.filter((e) => solo === null || e.id === solo);
    return { placed: lineupBuildings(shown), fallen: lineupFallen(shown, state) };
  }, [lineup, solo, state]);
  const buildingsFeed = useFeed<SideBuildings | null>(buildings);

  // The tier every building is drawn at (null: by distance).
  const { tier, style, drawAt } = useBuildingTier(query.tier);

  // The stations, each drawn exactly as given (a raw framing): a transition
  // stands at a range, not where the rig's limits or clearance would put it.
  const rig = useMemo(() => new CameraController(gameCamera.config), []);
  const cameraAt = useCallback(
    (station: Station, id: string | null): Camera3DParams => {
      const lens = {
        ...gameCamera.lens,
        aspect: window.innerWidth / Math.max(1, window.innerHeight),
      };
      const { yaw, distance } = gameCamera.opening();
      const entry = lineup.entries.find((e) => e.id === id) ?? lineup.entries[0];
      const framing = (min: readonly number[], max: readonly number[]) =>
        cameraToFit(min, max, yaw, rig.pitchAt(WIDE_M), lens, FILL);
      const orbit = (pose: CameraPose): Camera3DParams => ({
        ...lens,
        ...pose,
        target: [...pose.target, surfaceZ(...pose.target)],
      });
      const around = (target: [number, number], d: number) =>
        orbit({ target, distance: d, yaw, pitch: rig.pitchAt(d) });
      const middle: [number, number] = [
        (entry.min[0] + entry.max[0]) / 2,
        (entry.min[1] + entry.max[1]) / 2,
      ];
      switch (station) {
        case "all":
          return framing(
            [0, 1, 2].map((axis) => Math.min(...lineup.rows.map((r) => r.min[axis]))),
            [0, 1, 2].map((axis) => Math.max(...lineup.rows.map((r) => r.max[axis]))),
          );
        case "row": {
          const row = lineup.rows.find((r) => r.category === entry.category)!;
          return framing(row.min, row.max);
        }
        case "fit":
          return framing(entry.min, entry.max);
        case "close":
          return around([middle[0], entry.min[1]], CLOSE_M);
        case "tactical":
          return around(middle, distance);
        case "wide":
          return around(middle, WIDE_M);
        default: {
          const range = tierBoundaries()[Number(station.slice(-1)) - 1];
          return orbit(poseAtRange(entry, range, yaw, rig.pitchAt(range)));
        }
      }
    },
    [lineup, rig, surfaceZ],
  );
  const [station, setStation] = useState<Station>(query.station);
  const [at, setAt] = useState(
    lineup.entries.find((e) => e.id === query.at)?.id ?? lineup.entries[0].id,
  );
  const stand = useCallback(
    (next: Station, id: string | null = null) => {
      const entry = lineup.entries.find((e) => e.id === id);
      window.__lab?.setCamera?.(cameraAt(next, entry?.id ?? at));
      setStation(next);
      if (entry) setAt(entry.id);
    },
    [lineup, cameraAt, at],
  );

  // The labels: each template's id under the foot of its front, placed by
  // the live projection every frame.
  const [labels, setLabels] = useState(query.labels);
  const labelNodes = useRef(new Map<string, HTMLElement>());
  const placeLabels = useCallback(
    (project: Project) => {
      const inRow = new Map<string, number>();
      for (const entry of lineup.entries) {
        const place = inRow.get(entry.category) ?? 0;
        inRow.set(entry.category, place + 1);
        const node = labelNodes.current.get(entry.id);
        if (!node) continue;
        const [x, y] = [(entry.min[0] + entry.max[0]) / 2, entry.min[1] - LABEL_FRONT_M];
        const at = project(x, y, surfaceZ(x, y));
        node.style.display = at ? "" : "none";
        if (at)
          node.style.transform = `translate(${at[0]}px, ${at[1] + (place % 2) * LABEL_STAGGER_PX}px) translate(-50%, 0)`;
      }
    },
    [lineup, surfaceZ],
  );

  const costs = useMemo(
    () => (appearances.templates ? templateCosts(appearances.templates, appearances) : []),
    [appearances],
  );
  const diagnostics = useMemo(
    () => ({
      /** Where every template stands, and the catalogue templates without art. */
      lineup: () => ({ ...lineup, missing }),
      /** Per library template and state: rows and triangles at each tier. */
      costs: () => costs,
      /** Cut the camera to a station (of template `id`, or the one chosen). */
      stand,
      /** How far off each tier boundary is in this window, metres. */
      boundaries: tierBoundaries,
      /** One template alone where it stands (null: the whole line-up). */
      solo: setSolo,
      /** Every building at one tier (null: by distance); resolves once drawn. */
      drawAt,
      setState,
      setLabels,
      shown: () => ({ solo, state, tier, labels }),
      surfaceZ,
    }),
    [lineup, missing, costs, stand, drawAt, solo, state, tier, labels, surfaceZ],
  );

  if (!world || !meshes) return null;
  const available = TEMPLATE_STATES.filter(
    (s) => s === "intact" || costs.some((c) => c.states[s] !== undefined),
  );
  return (
    <>
      <LabViewport
        fixture="city-lineup"
        world={worldFeed}
        buildings={buildingsFeed}
        buildingStyle={style}
        appearances={kits}
        initialCamera={cameraAt(query.station, query.at)}
        groundAt={surfaceZ}
        onFrame={placeLabels}
        diagnostics={diagnostics}
      />
      {labels && (
        <div className="lineup-labels" data-testid="city-lineup-labels">
          {lineup.entries
            .filter((e) => solo === null || e.id === solo)
            .map((e) => (
              <span
                key={e.id}
                ref={(node) => {
                  if (node) labelNodes.current.set(e.id, node);
                  else labelNodes.current.delete(e.id);
                }}
              >
                {e.id}
              </span>
            ))}
        </div>
      )}
      <aside className="hud-panel lab-panel" data-testid="city-lineup-panel">
        <strong>Template line-up</strong>
        <div className="lab-hint">
          {lineup.entries.length} templates
          {query.set && ` of set ${query.set}`}
          {query.category && ` of category ${query.category}`}
          {missing.length > 0 && ` · ${missing.length} without art`}. WASD/arrows pan · Q/E turn ·
          middle‑drag orbit · wheel zoom
        </div>
        <div className="lab-row">
          {[null, 0, 1, 2, 3].map((t) => (
            <button
              key={String(t)}
              type="button"
              aria-pressed={t === tier}
              onClick={() => void drawAt(t)}
            >
              {t === null ? "auto tiers" : `tier ${t}`}
            </button>
          ))}
        </div>
        <div className="lab-row">
          {available.map((s) => (
            <button key={s} type="button" aria-pressed={s === state} onClick={() => setState(s)}>
              {s}
            </button>
          ))}
        </div>
        <label>
          Template{" "}
          <select
            value={at}
            onChange={(e) => stand(station === "all" ? "tactical" : station, e.target.value)}
          >
            {lineup.entries.map((e: LineupEntry) => (
              <option key={e.id} value={e.id}>
                {e.id}
              </option>
            ))}
          </select>
        </label>
        <div className="lab-row">
          {(["all", "row", ...TEMPLATE_STATIONS] as Station[]).map((s) => (
            <button key={s} type="button" aria-pressed={s === station} onClick={() => stand(s)}>
              {s}
            </button>
          ))}
        </div>
        <label>
          <input type="checkbox" checked={labels} onChange={(e) => setLabels(e.target.checked)} />
          Labels
        </label>
      </aside>
    </>
  );
}
