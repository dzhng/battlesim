// @vitest-environment node
// The info panels' rows, derived from the side's observation alone: every
// own unit state as an icon and a short word, and enemy panels that say only
// what the side can know.
import { expect, test } from "vitest";
import {
  contactPanel,
  enemyPanel,
  heardWeapons,
  ownStateRows,
  type PanelRules,
} from "../src/battle/present/panelRows";
import type { OwnUnitView } from "../src/battle/sim/observation";

// Fixed names and numbers, so the tests don't move when the fixture is
// tuned; the unit types (their mounts) are the shipped catalog's.
const RULES: PanelRules = {
  tick_hz: 30,
  weapons: {
    tank_ap: { name: "AP", icon: "ap_shell" },
    tank_he: { name: "HE", icon: "he_shell" },
    hmg: { name: "HMG", icon: "hmg" },
    rifle: { name: "Rifle", icon: "rifle" },
    grenade: { name: "Grenade", icon: "grenade" },
    atgm: { name: "ATGM", icon: "atgm" },
  },
  suppression: { collapse_level: 0.8 },
  service: { radius_m: 80 },
};

const unit = (u: Partial<OwnUnitView>): OwnUnitView =>
  ({
    id: 1,
    kind: "rifle",
    position: [0, 0, 0],
    state: "idle",
    suppression: 0,
    deployment: null,
    garrison: null,
    stock: null,
    service: "out_of_range",
    ...u,
  }) as OwnUnitView;
const words = (u: OwnUnitView, own: OwnUnitView[] = [u]) =>
  ownStateRows(u, own, RULES).map((r) => [r.word, r.progress]);

test("any unit that deploys shows the row from its published deployment, never its kind", () => {
  // A type the catalog doesn't know: the row reads the deployment alone.
  const gun = (progress: number, target: string) =>
    unit({ kind: "test_howitzer", deployment: { progress, target } });
  expect(words(gun(0.4, "deployed"))).toEqual([["DEPLOYING", 0.4]]);
  expect(words(gun(1, "deployed"))).toEqual([["DEPLOYED", null]]);
  // Packing fills as the packing completes.
  expect(words(gun(0.25, "packed"))).toEqual([["PACKING", 0.75]]);
  // Packed and staying packed: nothing worth saying.
  expect(words(gun(0, "packed"))).toEqual([]);
});

test("a unit in a set-up truck's reach says whether it is served, full or cannot be", () => {
  const at = (service: string) => words(unit({ service })).map(([w]) => w);
  expect(at("serving")).toEqual(["RESUPPLYING"]);
  expect(at("full")).toEqual(["SUPPLY FULL"]);
  for (const s of ["moving", "firing", "no_stock"]) expect(at(s), s).toEqual(["CANNOT SUPPLY"]);
  // Out of reach, or the truck isn't set up yet: no row.
  expect(at("out_of_range")).toEqual([]);
  expect(at("source_not_deployed")).toEqual([]);
  // The three families differ by their mark, not only their colour.
  const icons = ["serving", "full", "no_stock"].map(
    (service) => ownStateRows(unit({ service }), [], RULES)[0].icon,
  );
  expect(new Set(icons).size).toBe(3);
});

test("a truck shows its stock, and supplying while set up with a unit in reach being served", () => {
  const truck = unit({
    id: 1,
    kind: "supply",
    stock: 400,
    deployment: { progress: 1, target: "deployed" },
  });
  const served = unit({ id: 2, position: [60, 0, 0], service: "serving" });
  const far = unit({ id: 3, position: [200, 0, 0], service: "serving" });
  expect(words(truck, [truck]).map(([w]) => w)).toEqual(["DEPLOYED", "SUPPLY 400"]);
  expect(words(truck, [truck, far]).map(([w]) => w)).toEqual(["DEPLOYED", "SUPPLY 400"]);
  expect(words(truck, [truck, served]).map(([w]) => w)).toEqual([
    "DEPLOYED",
    "SUPPLY 400",
    "SUPPLYING",
  ]);
  // Deploying, it supplies nobody yet.
  const setting = { ...truck, deployment: { progress: 0.5, target: "deployed" } };
  expect(words(setting, [setting, served]).map(([w]) => w)).toEqual(["DEPLOYING", "SUPPLY 400"]);
});

test("suppression reads SUPPRESSED with its level, PINNED from the collapse level", () => {
  expect(words(unit({ suppression: 0.3 }))).toEqual([["SUPPRESSED", 0.3]]);
  expect(words(unit({ suppression: 0.8 }))).toEqual([["PINNED", 0.8]]);
  expect(words(unit({ suppression: 0.001 }))).toEqual([]);
});

test("a building's phases and waiting for the way ahead are rows", () => {
  const g = (phase: string, progress: number) =>
    words(unit({ garrison: { building: 3, phase, progress } }));
  expect(g("entering", 0.5)).toEqual([["ENTERING", 0.5]]);
  expect(g("waiting_for_room", 0)).toEqual([["NO ROOM", null]]);
  expect(g("inside", 1)).toEqual([["IN BUILDING", null]]);
  expect(g("exiting", 0.25)).toEqual([["LEAVING", 0.25]]);
  expect(words(unit({ state: "waiting" }))).toEqual([["WAITING", null]]);
});

test("an identified enemy's panel names its type and weapon types, never a count", () => {
  const tank = enemyPanel("tank", RULES);
  expect(tank.name).toBe("TANK");
  expect(tank.weapons.map((w) => w.label)).toEqual(["CANNON AP · HE", "HMG"]);
  const squad = enemyPanel("rifle", RULES);
  expect(squad.weapons.map((w) => w.label)).toEqual(["RIFLE", "GRENADE"]);
  // Nothing numeric: no ammunition, health or strength.
  for (const p of [tank, squad])
    expect(JSON.stringify([p.name, p.weapons.map((w) => w.label)])).not.toMatch(/\d/);
});

test("a last sighting keeps its identified name and weapons and counts up since", () => {
  const c = { source: "last_seen", kind: "tank", heard: [], evidenceTick: 300 };
  const p = contactPanel(c, 300 + 12 * 30 + 7, RULES);
  expect(p.name).toBe("TANK");
  expect(p.weapons.map((w) => w.label)).toEqual(["CANNON AP · HE", "HMG"]);
  expect(p.evidence?.word).toBe("LAST SEEN 12 s AGO");
  // The only number on it is the seconds.
  expect(JSON.stringify([p.name, p.weapons.map((w) => w.label)])).not.toMatch(/\d/);
});

test("a firing report is UNKNOWN, names what was heard by weapon type, and counts up since", () => {
  const c = { source: "firing", kind: null, heard: ["tank_ap", "tank_he"], evidenceTick: 90 };
  const p = contactPanel(c, 90 + 5 * 30, RULES);
  expect(p.name).toBe("UNKNOWN");
  expect(p.weapons.map((w) => w.label)).toEqual(["CANNON AP · HE"]);
  expect(p.evidence?.word).toBe("HEARD 5 s AGO");
  expect(heardWeapons(["hmg", "rifle"], RULES).map((w) => w.label)).toEqual(["HMG", "RIFLE"]);
  expect(contactPanel({ ...c, heard: [] }, 90, RULES).weapons).toEqual([]);
});
