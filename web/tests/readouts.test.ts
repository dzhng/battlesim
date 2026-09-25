// @vitest-environment node
import { expect, test } from "vitest";
import { ringAmmo, ringTimers, REASON_TEXT } from "../src/battle/present/readouts";
import type { MountView, OwnUnitView } from "../src/battle/sim/observation";

const mount = (m: Partial<MountView>): MountView => ({
  mount: 0,
  loaded: 0,
  ammo: [20, 15],
  aim: 1,
  reload: 0,
  target: null,
  reason: "no_compatible_target",
  guiding: false,
  reloading: null,
  ...m,
});
const tank = { kind: "tank" } as OwnUnitView;

test("completed timers vanish; running ones are the published fractions", () => {
  expect(ringTimers(mount({}))).toEqual({ aim: null, reload: null });
  const t = { kind: "identified" as const, id: 1 };
  expect(ringTimers(mount({ target: t, aim: 0.4, loaded: null, reload: 0.25 }))).toEqual({
    aim: 0.4,
    reload: 0.25,
  });
  // Aimed and loaded: nothing left to show.
  expect(ringTimers(mount({ target: t, aim: 1, loaded: 1, reload: 0 }))).toEqual({
    aim: null,
    reload: null,
  });
});

test("the cannon is one ring naming the loaded, else the reloading, kind", () => {
  expect(ringAmmo(tank, mount({ loaded: 0 }))).toBe("AP20");
  expect(ringAmmo(tank, mount({ loaded: null, reloading: 1 }))).toBe("HE15");
  expect(ringAmmo(tank, mount({ mount: 1, ammo: [null], loaded: 0 }))).toBe("∞");
});

test("every published action reason has player words", async () => {
  const { readFileSync } = await import("node:fs");
  const src = readFileSync(
    new URL("../../crates/contract/src/observation.rs", import.meta.url),
    "utf8",
  );
  const body = src.slice(
    src.indexOf("pub enum ActionReason"),
    src.indexOf("}", src.indexOf("pub enum ActionReason")),
  );
  const variants = [...body.matchAll(/^\s+([A-Z]\w+),/gm)].map((m) =>
    m[1].replace(/[A-Z]/g, (c, i) => (i ? "_" : "") + c.toLowerCase()),
  );
  expect(variants.length).toBeGreaterThan(10);
  for (const v of variants) expect(REASON_TEXT[v], v).toBeTruthy();
});
