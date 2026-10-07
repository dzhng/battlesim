// @vitest-environment node
import { expect, test } from "vitest";
import {
  CookOffWatch,
  LastSeenHulls,
  cookOffModels,
  transitionOf,
  type CookOffTransition,
} from "@apps/battle-lab/src/cookOffs";
import { gameEffects } from "@apps/battle-lab/src/effectFeed";
import { REST_ARTICULATION } from "@packages/scene-assets/src/articulation";
import type {
  ModelInstance,
  ResolveAppearance,
} from "@packages/battle-renderer/src/models/modelInstances";
import { UNITS } from "./catalog";
import type { ObservationView } from "@web/battle/sim/observation";
import { PropAppearances } from "@packages/battle-renderer/src/models/propAppearance";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import { landedAfter } from "@packages/battle-renderer/src/effects/cookOff";
import { effectCookOff } from "@apps/battle-lab/src/cookOffs";

const WRECK = UNITS.hull("test_tank")!.wreck;

test("a whole vehicle wreck keeps the live hull until the blast, then jolts and settles without a cut", () => {
  const wreck: ModelInstance = {
    appearance: "test_jeep_wreck",
    x: 40,
    y: 30,
    z: 0,
    yaw: 0.5,
    pose: { kind: "static", state: "default" },
  };
  const c = {
    prop: 3,
    kind: UNITS.hull("test_jeep")!.wreck,
    wreckOf: "test_jeep",
    center: [40, 30] as const,
    yaw: 0.5,
    half: [2.2, 1, 0.95] as const,
    baseZ: 0,
    tick: 301,
  };
  const fit = { fit: () => [wreck] } as unknown as PropAppearances;
  const installed = {
    appearances: new Map([
      [
        "test_jeep_wreck",
        {
          bundle: {
            kind: "static",
            states: [{ name: "default", bounds: { min: [-2.2, -1, 0], max: [2.2, 1, 2.8] } }],
          },
        },
      ],
    ]),
  } as unknown as InstalledAppearances;
  const transition = transitionOf(c, fit, installed, 30);
  expect(transition).not.toBeNull();
  const hull: ModelInstance = {
    ...wreck,
    appearance: "test_jeep",
    pose: { kind: "articulated", articulation: REST_ARTICULATION },
  };
  const feel = { ...gameEffects.cook_off, delay_s: 0.35 };
  const rolling = { model: { ...hull, x: 36 }, braking: 2 };
  expect(cookOffModels(transition!, rolling, feel, 10)[0].x).toBeCloseTo(36);
  expect(cookOffModels(transition!, rolling, feel, 11)[0].x).toBeCloseTo(39);
  expect(cookOffModels(transition!, rolling, feel, 12)[0].x).toBeCloseTo(40);
  expect(
    cookOffModels(transition!, { model: hull, braking: 2 }, feel, 10 + feel.delay_s / 2),
  ).toEqual([hull]);
  const moving = cookOffModels(
    transition!,
    { model: hull, braking: 2 },
    feel,
    10 + feel.delay_s + feel.settle_s / 8,
  );
  expect(moving[0].appearance).toBe("test_jeep_wreck");
  expect(moving[0].pose.kind).toBe("static");
  if (moving[0].pose.kind !== "static") throw new Error("expected moving wreck");
  expect(moving[0].pose.state).toBe("default");
  expect(moving[0].pose.motion![14]).not.toBe(0);
  // The complete wreck includes wheels and grounded debris: no corner may sink.
  for (const age of [0.4, 0.7, 1.1, 2.5]) {
    const m = cookOffModels(transition!, null, feel, 10 + age)[0];
    if (m.pose.kind !== "static") throw new Error("expected moving wreck");
    const motion = m.pose.motion!;
    for (const x of [-2.2, 2.2])
      for (const y of [-1, 1])
        for (const z of [0, 2.8])
          expect(
            motion[2] * x + motion[6] * y + motion[10] * z + motion[14],
          ).toBeGreaterThanOrEqual(-1e-7);
  }
  const settled = cookOffModels(transition!, null, feel, 10 + landedAfter(feel));
  if (settled[0].pose.kind !== "static") throw new Error("expected resting wreck");
  expect(settled[0].pose.state).toBe("default");
  const identity = [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1];
  settled[0].pose.motion!.forEach((v, i) => expect(v).toBeCloseTo(identity[i], 12));
});

/** A side's publication at `tick`: the enemy tank it identifies (if any), at
 *  `at`, and the wrecks it knows. */
function seen(
  tick: number,
  tank: readonly [number, number] | null,
  wrecks: { id: number; at: readonly [number, number] }[],
): ObservationView {
  return {
    tick,
    own: [],
    identified: tank
      ? [{ id: 7, kind: "test_tank", position: [tank[0], tank[1], 0], yaw: 0.5, weaponPoses: [] }]
      : [],
    knownProps: wrecks.map((w) => ({
      id: w.id,
      kind: WRECK,
      center: w.at,
      yaw: 0.5,
      half: [3.5, 1.8, 1.2],
      baseZ: 0,
      replaces: null,
      authoredProp: null,
      destroyed: false,
      wreckOf: "test_tank",
    })),
  } as unknown as ObservationView;
}

test("a tank the side watched die brews up where its wreck appears, once", () => {
  const watch = new CookOffWatch(UNITS, 30);
  expect(watch.note(seen(10, [40, 30], []))).toEqual([]);
  // The tank, seen moving, is a wreck the next tick: the side watched it go.
  const brewed = watch.note(seen(11, null, [{ id: 3, at: [40.3, 30] }]));
  expect(brewed.map((c) => [c.prop, c.tick, c.center])).toEqual([[3, 11, [40.3, 30]]]);
  // A wreck already known brews up no more.
  expect(watch.note(seen(12, null, [{ id: 3, at: [40.3, 30] }]))).toEqual([]);
});

test("the side learns the wreck a few ticks after it loses the hull, and still saw it die", () => {
  const watch = new CookOffWatch(UNITS, 30);
  watch.note(seen(92, [40, 30], []));
  for (const tick of [93, 94, 95]) expect(watch.note(seen(tick, null, []))).toEqual([]);
  expect(watch.note(seen(96, null, [{ id: 3, at: [40, 30] }])).map((c) => c.prop)).toEqual([3]);
  // A hull lost from sight long before its wreck is found was not watched dying.
  const late = new CookOffWatch(UNITS, 30);
  late.note(seen(10, [40, 30], []));
  late.note(seen(11, null, []));
  expect(late.note(seen(60, null, [{ id: 3, at: [40, 30] }]))).toEqual([]);
});

test("a wreck found later, or one no watched hull stood on, does not brew up", () => {
  const watch = new CookOffWatch(UNITS, 30);
  // Scouted onto an old wreck: no tank was seen there the tick before.
  watch.note(seen(10, [200, 30], []));
  expect(watch.note(seen(11, [200, 30], [{ id: 4, at: [40, 30] }]))).toEqual([]);
  // The first publication has no tick before it: what it knows is old news.
  expect(new CookOffWatch(UNITS, 30).note(seen(5, null, [{ id: 4, at: [40, 30] }]))).toEqual([]);
});

test("an earlier tick is a new battle: the watch starts over", () => {
  const watch = new CookOffWatch(UNITS, 30);
  watch.note(seen(10, [40, 30], []));
  watch.note(seen(11, null, [{ id: 3, at: [40, 30] }]));
  watch.note(seen(1, [40, 30], []));
  expect(watch.note(seen(2, null, [{ id: 3, at: [40, 30] }])).map((c) => c.prop)).toEqual([3]);
});

/** A red tank as the pose driver draws it, its turret trained `turret` off its hull. */
const tankPose = (at: readonly [number, number], turret: number) => ({
  unit: 7,
  kind: "test_tank",
  side: "red" as const,
  position: [at[0], at[1], 0] as [number, number, number],
  yaw: 0.5,
  articulation: { ...REST_ARTICULATION, turret_yaw: turret },
});

test("a Jeep death retains its own hull when another vehicle disappeared nearby", () => {
  const hulls = new LastSeenHulls(() => 2);
  hulls.note(
    [tankPose([40.5, 30], 1.2), { ...tankPose([40, 30], 0), unit: 8, kind: "test_jeep" }],
    10,
  );
  hulls.note([], 10.03);
  const c = {
    prop: 3,
    kind: UNITS.hull("test_jeep")!.wreck,
    wreckOf: "test_jeep",
    center: [40.5, 30] as const,
    yaw: 0.5,
    half: [2.2, 1, 0.95] as const,
    baseZ: 0,
    tick: 301,
  };
  const resolve: ResolveAppearance = (kind) => ({ appearance: kind, tint: [1, 0, 0] });
  expect(hulls.at(c, 10.1, resolve)?.model.appearance).toBe("test_jeep");
});

test("a hull that brews up is drawn whole, as last seen, until its ammunition goes; then as the wreck's pieces", () => {
  // A beat between the hit and the ammunition going, whatever the shipped feel has.
  const feel = { ...gameEffects.cook_off, delay_s: 0.35 };
  const resolve: ResolveAppearance = (kind) => ({ appearance: kind, tint: [1, 0, 0] });
  const hulls = new LastSeenHulls(() => 2);
  hulls.note([tankPose([40, 30], 1.2)], 10);
  hulls.note([], 10.03);
  const wreck: ModelInstance = {
    appearance: WRECK,
    x: 40.3,
    y: 30,
    z: 0,
    yaw: 0.5,
    pose: { kind: "static", state: "default" },
  };
  const transition: CookOffTransition = {
    cookOff: {
      prop: 3,
      kind: WRECK,
      wreckOf: "test_tank",
      center: [40.3, 30],
      yaw: 0.5,
      half: [3.5, 1.8, 1.2],
      baseZ: 0,
      tick: 301,
    },
    wreck,
    bounds: { min: [-2.2, -1, 0], max: [2.2, 1, 2.8] },
    lies: [0, 0, 1.6],
    underside: 1.2,
    hitAt: 10,
  };
  const hull = hulls.at(transition.cookOff, 10.1, resolve);
  // Before the ammunition goes: the clean tank, turret where it was trained.
  const before = cookOffModels(transition, hull, feel, 10);
  expect(before.map((m) => [m.appearance, m.x, m.y, m.yaw])).toEqual([["test_tank", 40, 30, 0.5]]);
  expect(before[0].pose.kind === "articulated" && before[0].pose.articulation.turret_yaw).toBe(1.2);
  // Once it goes: the wreck's hull and its moving turret.
  const after = cookOffModels(transition, hull, feel, 10 + feel.delay_s + 0.05);
  expect(after.map((m) => [m.appearance, m.pose.kind === "static" && m.pose.state])).toEqual([
    [WRECK, "hull"],
    [WRECK, "turret"],
  ]);
  // A hull long gone, or one that stood elsewhere, is not the one that brewed up.
  expect(hulls.at(transition.cookOff, 15, resolve)).toBeNull();
  expect(hulls.at({ ...transition.cookOff, center: [200, 30] }, 10.1, resolve)).toBeNull();
});

test("a hull killed on the move glides on to its wreck, slowing as it brakes, and its pieces follow", () => {
  const feel = gameEffects.cook_off;
  const resolve: ResolveAppearance = (kind) => ({ appearance: kind, tint: [1, 0, 0] });
  const hulls = new LastSeenHulls(() => 2);
  // Last seen 4 m short of where its wreck came to rest.
  hulls.note([tankPose([36, 30], 0)], 10);
  hulls.note([], 10.03);
  const wreck: ModelInstance = {
    appearance: WRECK,
    x: 40,
    y: 30,
    z: 0,
    yaw: 0,
    pose: { kind: "static", state: "default" },
  };
  const transition: CookOffTransition = {
    cookOff: {
      prop: 3,
      kind: WRECK,
      wreckOf: "test_tank",
      center: [40, 30],
      yaw: 0,
      half: [3.5, 1.8, 1.2],
      baseZ: 0,
      tick: 301,
    },
    wreck,
    bounds: { min: [-2.2, -1, 0], max: [2.2, 1, 2.8] },
    lies: [0, 0, 1.6],
    underside: 1.2,
    hitAt: 10,
  };
  // Braking at 2 m/s²: 4 m takes 2 s to roll.
  const hull = hulls.at(transition.cookOff, 10.05, resolve);
  const xAt = (clock: number) => cookOffModels(transition, hull, feel, clock)[0].x;
  expect(xAt(10)).toBeCloseTo(36);
  // Half its time, three quarters of its way: it slows as it goes.
  expect(xAt(11)).toBeCloseTo(39);
  expect(xAt(12)).toBeCloseTo(40);
  expect(xAt(14)).toBeCloseTo(40);
  // The wreck's pieces, once the ammunition goes, are where the hull has rolled to.
  const pieces = cookOffModels(transition, hull, feel, 11);
  expect(pieces.every((m) => Math.abs(m.x - 39) < 1e-9)).toBe(true);
});

test("a cook-off throws a turret only from a wreck with a turret piece: never a wheeled one's", () => {
  // The tank's wreck is cut into a hull and a turret; the jeep's is whole.
  const states = (...names: string[]) =>
    names.map((name) => ({ name, bounds: { min: [-2, -1, 0], max: [2, 1, 2] }, tiers: [] }));
  const appearance = (scenery: string | null, footprint: number[] | null, wreck: string | null, ...names: string[]) => ({
    unit: scenery ? "scenery" : "vehicle",
    scenery,
    footprint,
    mounts: null,
    regionalFamily: null,
    paints: null,
    wreck,
    bundle: { kind: "static", states: states(...names), materials: [], textures: [] },
  });
  const installed = {
    appearances: new Map([
      ["test_tank", appearance(null, null, "test_tank_wreck")],
      ["test_jeep", appearance(null, null, "test_jeep_wreck")],
      ["test_tank_wreck", appearance("wreck", [3.5, 1.8, 1.2], null, "default", "hull", "turret")],
      ["test_jeep_wreck", appearance("wreck", [2.2, 1, 0.95], null, "default")],
    ]),
  } as unknown as InstalledAppearances;
  const fit = new PropAppearances(
    installed,
    {
      propAppearance: Object.fromEntries(
        Object.entries(UNITS.view.props).map(([id, t]) => [id, t.appearance]),
      ),
      blockingPropKinds: {},
      unitAppearance: { test_tank: "test_tank", test_jeep: "test_jeep" },
    },
    null,
  );
  const died = (unit: string, half: readonly [number, number, number]) => ({
    prop: 3,
    kind: UNITS.hull(unit)!.wreck,
    wreckOf: unit,
    center: [40, 30] as const,
    yaw: 0.5,
    half,
    baseZ: 0,
    tick: 301,
  });
  const jeep = died("test_jeep", [2.2, 1, 0.95]);
  const whole = transitionOf(jeep, fit, installed, 30);
  expect(whole?.wreck.appearance).toBe("test_jeep_wreck");
  expect(whole?.lies).toBeNull();
  expect(effectCookOff(jeep, whole).landing).toBeNull();
  // The tank's own wreck has the piece: its turret is thrown and lands.
  const tank = died("test_tank", [3.5, 1.8, 1.2]);
  const thrown = transitionOf(tank, fit, installed, 30);
  expect(thrown?.wreck.appearance).toBe("test_tank_wreck");
  expect(effectCookOff(tank, thrown).landing).not.toBeNull();
});

test("a hull brews up into its own unit's wreck, not another unit's lying beside it", () => {
  // The side loses its jeep beside a tank's fresh wreck: no brew-up there.
  const watch = new CookOffWatch(UNITS, 30);
  const jeepAt = (tick: number, alive: boolean): ObservationView =>
    ({
      ...seen(tick, null, alive ? [] : [{ id: 3, at: [40.3, 30] }]),
      identified: alive
        ? [{ id: 8, kind: "test_jeep", position: [40, 30, 0], yaw: 0.5, weaponPoses: [] }]
        : [],
    }) as unknown as ObservationView;
  watch.note(jeepAt(10, true));
  expect(watch.note(jeepAt(11, false))).toEqual([]);
});
