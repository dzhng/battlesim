// @vitest-environment node
import { expect, test } from "vitest";
import { easeNudge } from "../src/battle/present/readouts";
import { REASON_MARK, REASON_TEXT } from "../src/battle/present/infoPanel";
import { mountTimers, ownStateRows, type PanelRules } from "../src/battle/present/panelRows";
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
test("completed timers vanish; running ones are the published fractions", () => {
  expect(mountTimers(mount({}))).toEqual({ aim: null, reload: null });
  const t = { kind: "identified" as const, id: 1 };
  expect(mountTimers(mount({ target: t, aim: 0.4, loaded: null, reload: 0.25 }))).toEqual({
    aim: 0.4,
    reload: 0.25,
  });
  // Aimed and loaded: nothing left to show.
  expect(mountTimers(mount({ target: t, aim: 1, loaded: 1, reload: 0 }))).toEqual({
    aim: null,
    reload: null,
  });
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

test("every published action reason has player words and a mark, and every garrison phase a row", async () => {
  const reasons = await contractEnum("ActionReason");
  expect(reasons.length).toBeGreaterThan(10);
  for (const v of reasons) expect(REASON_TEXT[v], v).toBeTruthy();
  // And a panel mark, or a decision to show none.
  for (const v of reasons) expect(REASON_MARK[v], v).not.toBeUndefined();
  const phases = await contractEnum("GarrisonPhase");
  expect(phases.length).toBeGreaterThan(2);
  const rules = { tick_hz: 30, weapons: {}, service: { radius_m: 1 } } as PanelRules;
  for (const phase of phases) {
    const u = { garrison: { phase, progress: 0.5 }, suppression: "none", service: "", stock: null };
    expect(ownStateRows(u as unknown as OwnUnitView, [], rules), phase).toHaveLength(1);
  }
});

test("a callout eases to a new nudge over about 150 ms of presentation clock, and snaps under a held one", () => {
  const from = { dx: 0, dy: 0 },
    to = { dx: 0, dy: -40 };
  // A frame at 60 Hz moves it part of the way, never past.
  const step = easeNudge(from, to, 1 / 60);
  expect(step.dy).toBeLessThan(0);
  expect(step.dy).toBeGreaterThan(-40);
  // Nine such frames (150 ms) put it within a few pixels.
  let n = from;
  for (let k = 0; k < 9; k++) n = easeNudge(n, to, 1 / 60);
  expect(Math.abs(n.dy - to.dy)).toBeLessThan(3);
  // A held clock, a rewound one or a first placement: the settled layout.
  expect(easeNudge(from, to, 0)).toEqual(to);
  expect(easeNudge(from, to, -1)).toEqual(to);
  expect(easeNudge(undefined, to, 1 / 60)).toEqual(to);
});
