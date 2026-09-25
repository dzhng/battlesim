// @vitest-environment node
import { expect, test } from "vitest";
import {
  GARRISON_PHASE_TEXT,
  garrisonText,
  ringAmmo,
  ringTimers,
  REASON_TEXT,
  SERVICE_TEXT,
  unitStrength,
} from "../src/battle/present/readouts";
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

/** The snake_case variants of a contract enum, read from the Rust source. */
async function contractEnum(name: string): Promise<string[]> {
  const { readFileSync } = await import("node:fs");
  const src = readFileSync(
    new URL("../../crates/contract/src/observation.rs", import.meta.url),
    "utf8",
  );
  const at = src.indexOf(`pub enum ${name}`);
  const body = src.slice(at, src.indexOf("}", at));
  return [...body.matchAll(/^\s+([A-Z]\w+),/gm)].map((m) =>
    m[1].replace(/[A-Z]/g, (c, i) => (i ? "_" : "") + c.toLowerCase()),
  );
}

test("every published action reason, service state and garrison phase has player words", async () => {
  const reasons = await contractEnum("ActionReason");
  expect(reasons.length).toBeGreaterThan(10);
  for (const v of reasons) expect(REASON_TEXT[v], v).toBeTruthy();
  const service = await contractEnum("ServiceStatus");
  expect(service.length).toBeGreaterThan(4);
  for (const v of service) expect(SERVICE_TEXT[v], v).toBeTruthy();
  const phases = await contractEnum("GarrisonPhase");
  expect(phases.length).toBeGreaterThan(2);
  for (const v of phases) expect(GARRISON_PHASE_TEXT[v], v).toBeTruthy();
});

test("strength counts a squad's losses as well as its wounds", () => {
  const squad = (memberHp: number[]) =>
    ({
      kind: "rifle",
      members: memberHp.map(() => [0, 0, 0]),
      memberHp,
      hp: 0,
    }) as unknown as OwnUnitView;
  // Eight soldiers at full health, then four left at half.
  expect(unitStrength(squad(Array(8).fill(100)))).toBe(1);
  expect(unitStrength(squad(Array(4).fill(50)))).toBe(0.25);
  expect(unitStrength({ ...tank, members: [], hp: 40 } as OwnUnitView)).toBe(0.4);
});

test("a garrison timer shows while entering or leaving", () => {
  const at = (phase: string, progress: number) =>
    ({ garrison: { phase, progress } }) as unknown as OwnUnitView;
  expect(garrisonText(at("entering", 0.25))).toBe("entering 25%");
  expect(garrisonText(at("inside", 1))).toBe("inside");
  expect(garrisonText({ garrison: null } as unknown as OwnUnitView)).toBe("outside");
});
