// @vitest-environment node
import { expect, test } from "vitest";
import {
  CookOffWatch,
  LastSeenHulls,
  cookOffModels,
  type Flight,
} from "@apps/battle-lab/src/cookOffs";
import { gameEffects } from "@apps/battle-lab/src/effectFeed";
import { REST_ARTICULATION } from "@packages/scene-assets/src/articulation";
import type {
  ModelInstance,
  ResolveAppearance,
} from "@packages/battle-renderer/src/models/modelInstances";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import type { ObservationView } from "@web/battle/sim/observation";

const WRECK = UNITS.hull("tank")!.wreck;

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
      ? [{ id: 7, kind: "tank", position: [tank[0], tank[1], 0], yaw: 0.5, weaponPoses: [] }]
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
  kind: "tank",
  side: "red" as const,
  position: [at[0], at[1], 0] as [number, number, number],
  yaw: 0.5,
  articulation: { ...REST_ARTICULATION, turret_yaw: turret },
});

test("a hull that brews up is drawn whole, as last seen, until its ammunition goes; then as the wreck's pieces", () => {
  const feel = gameEffects.cook_off;
  const resolve: ResolveAppearance = (kind) => ({ appearance: kind, tint: [1, 0, 0] });
  const hulls = new LastSeenHulls();
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
  const flight: Flight = {
    cookOff: {
      prop: 3,
      kind: WRECK,
      center: [40.3, 30],
      yaw: 0.5,
      half: [3.5, 1.8, 1.2],
      baseZ: 0,
      tick: 301,
    },
    wreck,
    lies: [0, 0, 1.6],
    underside: 1.2,
    hitAt: 10,
  };
  const hull = hulls.at(flight.cookOff, 10.1, resolve);
  // Before the ammunition goes: the clean tank, turret where it was trained.
  const before = cookOffModels(flight, hull, feel, 10 + feel.delay_s / 2);
  expect(before.map((m) => [m.appearance, m.x, m.y, m.yaw])).toEqual([["tank", 40, 30, 0.5]]);
  expect(before[0].pose.kind === "articulated" && before[0].pose.articulation.turret_yaw).toBe(1.2);
  // Once it goes: the wreck's hull and its thrown turret.
  const after = cookOffModels(flight, hull, feel, 10 + feel.delay_s + 0.05);
  expect(after.map((m) => [m.appearance, m.pose.kind === "static" && m.pose.state])).toEqual([
    [WRECK, "hull"],
    [WRECK, "turret"],
  ]);
  // A hull long gone, or one that stood elsewhere, is not the one that brewed up.
  expect(hulls.at(flight.cookOff, 15, resolve)).toBeNull();
  expect(hulls.at({ ...flight.cookOff, center: [200, 30] }, 10.1, resolve)).toBeNull();
});
