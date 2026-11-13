// The showcase lab: units at rest with no battle, side by side on flat
// ground, each on its own `ShowcaseStage` with its art from its own
// `ArmoryAppearances` (as the Armory mounts them), so idle life is judged as
// the Armory shows it. The three are chosen by
// property, never by id: the first tracked hull, the first rotorcraft and the
// first squad with a special mount. `?unit=` shows one kind alone,
// `?faction=` the look it wears, and `?yard=1` stands each unit on its spot in
// the Armory's yard (`armory/yard.ts`), seen from the spot's camera. Probes
// for `web/scenes/showcase.mjs` hang on `window.__showcase`.
import { useEffect, useMemo, useRef } from "react";
import { vec3, type Vec3 } from "math";
import { FACTIONS, type Faction, type UnitCatalog } from "@packages/scene-assets/src/units";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { EFFECT_WORDS, SHAPE_NAMES } from "@packages/battle-renderer/src/records.gen";
import type { IdleLoop } from "@packages/battle-audio/src/soundFrame";
import type { WorldLayers } from "@packages/battle-renderer/src/scene";
import { useSessionCatalog } from "@web/battle/catalog/context";
import { ShowcaseStage, type StageArt, type StageFielding } from "@web/battle/render/showcaseStage";
import { appResources } from "../appResources";
import { gameCamera } from "../gameCamera";
import { presentedCamera } from "../presentedFrame";
import { plainWorld } from "../plainWorld";
import { parkedAircraft } from "../showcaseAircraft";
import { ArmoryAppearances } from "../armory/armoryAppearances";
import { STAGE_LOOK, SQUAD_SEED, useShowground, useStageFielding } from "../armory/stageInputs";
import { drawnModels, grounded, project } from "../armory/stageProbes";
import { spotFor, yard, yardModels, yardScenery, yardWorld, type Spot } from "../armory/yard";
import type { Showground } from "@wasm/game_wasm.js";

/** Plain level ground, one colour, shadows read on it. */
const WORLD = plainWorld();

/** The kinds the lab shows by default, each the first of the catalog's
 *  types with a property; in the yard, a fixed-wing aircraft besides. */
function propertyPicks(units: UnitCatalog, inYard: boolean): string[] {
  const types = units.view.units;
  const flies = (t: (typeof types)[number], flight: string) =>
    "air" in t.mobility && t.mobility.air.flight === flight;
  const picks = [
    types.find((t) => "tracked" in t.mobility && "hull" in t.body),
    types.find((t) => flies(t, "rotor")),
    inYard ? types.find((t) => flies(t, "fixed_wing")) : undefined,
    types.find((t) => "squad" in t.body && t.mounts.some((m) => m.special)),
  ];
  return picks.filter((t) => t !== undefined).map((t) => t.id);
}

/** Where a panel's camera starts: in the yard, its spot's camera; else a
 *  little above an aircraft's side, its rotors in frame; 30° above a squad,
 *  its spread in frame; a three-quarter view of a hull. */
function opening(units: UnitCatalog, kind: string, spot: Spot | null): Camera3DParams {
  if (spot) return { ...gameCamera.lens, ...spot.camera };
  const hull = units.hull(kind);
  const air = "air" in units.type(kind).mobility;
  // A squad stands scattered over about twelve metres; a hull's gun can
  // reach well past its box.
  const [hx, hy, hz] = hull?.half_extents_m ?? [6, 6, 0.9];
  return {
    ...gameCamera.lens,
    target: [0, 0, hz],
    distance: (hull ? (air ? 3.6 : 6.5) : 2.9) * Math.max(hx, hy) + 3,
    pitch: hull ? (air ? 0.16 : 0.32) : Math.PI / 6,
    yaw: air ? -Math.PI / 2 : -0.75,
  };
}

interface Panel {
  kind: string;
  stage: ShowcaseStage;
  canvas: HTMLCanvasElement;
  camera: () => ReturnType<typeof presentedCamera>;
  /** The orbit the camera looks from now. */
  orbit: () => Camera3DParams;
  loops: IdleLoop[];
  art: StageArt;
}

/** A failure no panel can recover from: logged, then the page's resources
 *  are refused, as a battle's are. */
function refuse(error: unknown) {
  console.error("showcase:", error);
  appResources.refuse(error);
}

/** Every live panel, by kind, for the probes. */
const panels = new Map<string, Panel>();
let paused = false;

export default function Showcase() {
  const catalog = useSessionCatalog();
  const { units } = catalog;
  const search = new URLSearchParams(window.location.search);
  const asked = search.get("unit");
  const faction: Faction = FACTIONS.find((f) => f === search.get("faction")) ?? FACTIONS[0];
  const inYard = search.get("yard") === "1";
  const kinds = useMemo(
    () => (asked && units.has(asked) ? [asked] : propertyPicks(units, inYard)),
    [asked, units, inYard],
  );
  const factions = useMemo(() => new Set<string>([faction]), [faction]);
  const spots = useMemo(
    () => new Map(kinds.map((k) => [k, inYard ? spotFor(yard, units.type(k)) : null])),
    [inYard, kinds, units],
  );
  const fielding = useStageFielding(catalog);
  const showground = useShowground(fielding.rules, refuse);
  useEffect(() => {
    (window as unknown as { __showcase: unknown }).__showcase = {
      kinds: () => [...panels.keys()],
      ready: () => kinds.every((k) => panels.has(k)),
      pause: (on: boolean) => (paused = on),
      draw: (time: number) => [...panels.values()].every((p) => p.stage.frame(time, p.camera())),
      rect: (kind: string) => rectOf(panels.get(kind)!),
      loops: (kind: string) => panels.get(kind)?.loops ?? null,
      rotors: (kind: string) => rotorDiscs(panels.get(kind)!),
      models: (kind: string) => drawnModels(panels.get(kind)!.stage),
      standing: (kind: string) => panels.get(kind)?.stage.standing() ?? null,
      grounded: (kind: string) => {
        const p = panels.get(kind)!;
        return grounded(drawnModels(p.stage), p.art.appearances);
      },
      carried: (kind: string) =>
        units.type(kind).mounts.map((m) => [...(m.operator_appearance?.carried ?? [])]),
      parkedAircraft: () => parkedAircraft(units, factions),
      spot: (kind: string) => spots.get(kind) ?? null,
      yardModels: () => {
        const art = [...panels.values()][0]?.art;
        return inYard && art
          ? yardModels(yard, units.view.props, art.appearances).map((m) => ({
              name: m.appearance,
              at: [m.x, m.y],
            }))
          : [];
      },
    };
  }, [kinds, units, factions, spots, inYard]);

  if (!showground) return <div className="lab-panel">Loading units…</div>;
  return (
    <div className="showcase-lab" data-testid="showcase-lab">
      {kinds.map((kind) => {
        const spot = spots.get(kind) ?? null;
        return (
          <ShowcasePanel
            key={`${kind}:${faction}`}
            kind={kind}
            faction={faction}
            spot={spot}
            camera={opening(units, kind, spot)}
            world={inYard ? yardWorld() : WORLD}
            units={units}
            inYard={inYard}
            fielding={fielding}
            showground={showground}
          />
        );
      })}
    </div>
  );
}

function ShowcasePanel(props: {
  kind: string;
  faction: Faction;
  /** Where it stands in the yard; null: at the origin, on plain ground. */
  spot: Spot | null;
  camera: Camera3DParams;
  world: WorldLayers;
  units: UnitCatalog;
  /** Dressed with the yard's props. */
  inYard: boolean;
  fielding: StageFielding;
  showground: Showground;
}) {
  const { kind, faction, spot, world, units, inYard, fielding, showground } = props;
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const orbit = useRef(props.camera);
  useEffect(() => {
    const canvas = canvasRef.current!;
    let live = true;
    let raf = 0;
    let stage: ShowcaseStage | null = null;
    const camera = () => {
      const dpr = window.devicePixelRatio || 1;
      const width = Math.max(1, Math.round(canvas.clientWidth * dpr));
      const height = Math.max(1, Math.round(canvas.clientHeight * dpr));
      if (canvas.width !== width || canvas.height !== height)
        [canvas.width, canvas.height] = [width, height];
      return presentedCamera({ camera3d: orbit.current, width, height });
    };
    const appearances = new ArmoryAppearances(units);
    void (async () => {
      const gpu = await appResources.gpu();
      const { props } = units.view;
      const scenery = inYard
        ? yardScenery(yard, props, await appearances.catalog())
        : new Set<string>();
      const art = await appearances.show({ kind, faction, scenery });
      if (!live) return;
      const made = ShowcaseStage.create({
        gpu,
        canvas,
        world,
        models: (installed) => (inYard ? yardModels(yard, props, installed) : []),
        fielding,
        showground,
        look: STAGE_LOOK,
        failed: refuse,
      });
      stage = made;
      const [x, y] = spot?.at ?? [0, 0];
      const unit = { kind, faction, at: [x, y, 0] as Vec3, yaw: spot?.yaw ?? 0 };
      const loops = await made.show(unit, SQUAD_SEED, art);
      if (!live) return;
      const orbitNow = () => orbit.current;
      panels.set(kind, { kind, stage: made, canvas, camera, orbit: orbitNow, loops, art });
      const start = performance.now();
      const loop = (now: number) => {
        if (!live) return;
        if (!paused) made.frame((now - start) / 1000, camera());
        raf = requestAnimationFrame(loop);
      };
      raf = requestAnimationFrame(loop);
    })().catch(refuse);
    return () => {
      live = false;
      cancelAnimationFrame(raf);
      panels.delete(kind);
      stage?.dispose();
      appearances.dispose();
    };
  }, [kind, faction, spot, world, units, inYard, fielding, showground]);

  const drag = useRef<[number, number] | null>(null);
  return (
    <figure className="showcase-panel">
      <canvas
        ref={canvasRef}
        data-testid={`showcase-${kind}`}
        onPointerDown={(e) => {
          drag.current = [e.clientX, e.clientY];
          e.currentTarget.setPointerCapture(e.pointerId);
        }}
        onPointerMove={(e) => {
          if (!drag.current) return;
          const [x, y] = drag.current;
          drag.current = [e.clientX, e.clientY];
          const o = orbit.current;
          const pitch = Math.min(1.4, Math.max(0.02, o.pitch + (e.clientY - y) * 0.006));
          orbit.current = { ...o, yaw: o.yaw - (e.clientX - x) * 0.008, pitch };
        }}
        onPointerUp={() => (drag.current = null)}
        onWheel={(e) => {
          const o = orbit.current;
          orbit.current = { ...o, distance: o.distance * Math.exp(e.deltaY * 0.001) };
        }}
      />
      <figcaption>{kind}</figcaption>
    </figure>
  );
}

/** A panel's canvas on the page, in CSS pixels. */
function rectOf(p: Panel) {
  const r = p.canvas.getBoundingClientRect();
  return { x: r.left, y: r.top, width: r.width, height: r.height };
}

/** Screen position (CSS pixels within the panel) of world point `at`. */
function projected(p: Panel, at: Vec3): [number, number] | null {
  const { width, height } = rectOf(p);
  return project(p.orbit(), width, height, at);
}

/** Each rotor disc drawn last frame: its hub on screen and its radius in
 *  pixels (the larger of its reach across and along the view). */
function rotorDiscs(p: Panel) {
  const { frame } = p.stage.records();
  const f = frame.effects.f32;
  const rotor = SHAPE_NAMES.indexOf("rotor");
  const discs: { at: [number, number]; radius: number }[] = [];
  for (let i = 0; i < frame.effects.count; i++) {
    const w = i * EFFECT_WORDS;
    // Each rotor's blades: the one record with a chord.
    if (f[w + 12] !== rotor || !(f[w + 13] > 0)) continue;
    const hub: Vec3 = [f[w], f[w + 1], f[w + 2]];
    const r = f[w + 3];
    const at = projected(p, hub);
    if (!at) continue;
    const reach = [
      [r, 0, 0],
      [0, r, 0],
      [0, 0, r],
    ].map((o) => {
      const q = projected(p, vec3.add(vec3.create(), hub, o as Vec3));
      return q ? Math.hypot(q[0] - at[0], q[1] - at[1]) : 0;
    });
    discs.push({ at, radius: Math.max(...reach) });
  }
  return discs;
}
