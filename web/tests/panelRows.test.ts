// @vitest-environment node
// The info panels' rows, derived from the side's observation alone: every
// own unit state as an icon and a short word, and enemy panels that say only
// what the side can know.
import { expect, test } from "vitest";
import {
  contactPanel,
  enemyPanel,
  heardWeapons,
  ownPanel,
  ownStateRows,
  pipsLit,
  weaponLabel,
  type PanelRules,
} from "../src/battle/present/panelRows";
import type { MountView, OwnUnitView } from "../src/battle/sim/observation";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";

// Fixed names and numbers, so the tests don't move when the fixture is
// tuned; the unit types (their mounts) are the shipped catalog's.
const RULES: PanelRules = {
  tick_hz: 30,
  weapons: {
    tank_ap: { name: "AP", icon: "ap_shell", ammo: 20 },
    tank_he: { name: "HE", icon: "he_shell", ammo: 10 },
    hmg: { name: "HMG", icon: "hmg", ammo: "unlimited" },
    rifle: { name: "Rifle", icon: "rifle", ammo: "unlimited" },
    grenade: { name: "Grenade", icon: "grenade", ammo: 8 },
    atgm: { name: "ATGM", icon: "atgm", ammo: 4 },
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
  // Deploying, it supplies nobody yet; its lasting stock stays above.
  const setting = { ...truck, deployment: { progress: 0.5, target: "deployed" } };
  expect(words(setting, [setting, served]).map(([w]) => w)).toEqual(["SUPPLY 400", "DEPLOYING"]);
  // An empty truck's stock is a warning of its own.
  expect(ownStateRows({ ...truck, stock: 0 }, [], RULES).map((r) => r.state)).toContain(
    "stock_empty",
  );
});

test("suppression is its word alone: SUPPRESSED, and PINNED from the collapse level in its own hotter tone", () => {
  const rows = (s: number) =>
    ownStateRows(unit({ suppression: s }), [], RULES).map((r) => [
      r.word,
      r.progress,
      r.fill,
      r.tone,
    ]);
  expect(rows(0.3)).toEqual([["SUPPRESSED", null, null, "warn"]]);
  expect(rows(0.8)).toEqual([["PINNED", null, null, "pinned"]]);
  expect(rows(0.001)).toEqual([]);
});

test("lasting states come first, then what is happening now", () => {
  const squad = unit({
    garrison: { building: 3, phase: "inside", progress: 1 },
    suppression: 0.4,
    service: "serving",
  } as Partial<OwnUnitView>);
  expect(words(squad).map(([w]) => w)).toEqual(["IN BUILDING", "SUPPRESSED", "RESUPPLYING"]);
});

test("an amount lights its share of five pips: any at all one, none none", () => {
  expect([0, 0.01, 0.2, 0.5, 0.85, 1].map(pipsLit)).toEqual([0, 1, 1, 3, 4, 5]);
});

test("a truck's stock and a weapon's loaded kind fill against their full amounts; unlimited has none", () => {
  const truck = unit({ kind: "supply", stock: 150 });
  const full = UNITS.type("supply").capabilities.supply!.stock;
  expect(ownStateRows(truck, [], RULES).find((r) => r.state === "stock")?.fill).toBeCloseTo(
    150 / full,
    9,
  );
  const tank = unit({
    kind: "tank",
    mounts: [
      mount({ mount: 0, ammo: [17, 5], loaded: null, reloading: 1 }),
      mount({ mount: 1, ammo: [null] }),
    ],
  });
  const [cannon, hmg] = ownPanel(tank, [], RULES).weapons;
  // The kind being loaded: HE, 5 of 10.
  expect(cannon.fill).toBe(0.5);
  expect(hmg.fill).toBeNull();
  const empty = ownPanel(
    unit({ kind: "rifle", mounts: [mount({}), mount({ mount: 1, ammo: [0], loaded: null })] }),
    [],
    RULES,
  ).weapons[1];
  expect(empty.fill).toBe(0);
  expect(enemyPanel("tank", RULES).weapons.every((w) => w.fill === null)).toBe(true);
});

test("a building's phases and waiting for the way ahead are rows", () => {
  const g = (phase: string, progress: number) =>
    words(unit({ garrison: { building: 3, phase, progress, center: [0, 0], half: [5, 5] } }));
  expect(g("entering", 0.5)).toEqual([["ENTERING", 0.5]]);
  expect(g("waiting_for_room", 0)).toEqual([["NO ROOM", null]]);
  expect(g("inside", 1)).toEqual([["IN BUILDING", null]]);
  expect(g("exiting", 0.25)).toEqual([["LEAVING", 0.25]]);
  expect(words(unit({ state: "waiting" }))).toEqual([["WAITING", null]]);
  // No known route is its own warning, apart from waiting on traffic.
  expect(
    ownStateRows(unit({ state: "route_blocked" }), [], RULES).map((r) => [r.word, r.tone]),
  ).toEqual([["ROUTE BLOCKED", "warn"]]);
});

test("an identified enemy's panel names its type and weapon types, never a count", () => {
  const tank = enemyPanel("tank", RULES);
  expect(tank.name).toBe("TANK");
  expect(tank.weapons.map(weaponLabel)).toEqual(["CANNON AP · HE", "HMG"]);
  const squad = enemyPanel("rifle", RULES);
  expect(squad.weapons.map(weaponLabel)).toEqual(["RIFLE", "GRENADE"]);
  // Nothing numeric: no ammunition, health or strength.
  for (const p of [tank, squad])
    expect(JSON.stringify([p.name, p.weapons.map(weaponLabel)])).not.toMatch(/\d/);
});

test("a last sighting keeps its identified name and weapons and counts up since", () => {
  const c = { source: "last_seen", kind: "tank", heard: [], evidenceTick: 300 };
  const p = contactPanel(c, 300 + 12 * 30 + 7, RULES);
  expect(p.name).toBe("TANK");
  expect(p.weapons.map(weaponLabel)).toEqual(["CANNON AP · HE", "HMG"]);
  expect(p.states.at(-1)?.word).toBe("LAST SEEN 12 s AGO");
  // The only number on it is the seconds.
  expect(JSON.stringify([p.name, p.weapons.map(weaponLabel)])).not.toMatch(/\d/);
});

test("a firing report is UNKNOWN, names what was heard by weapon type, and counts up since", () => {
  const c = { source: "firing", kind: null, heard: ["tank_ap", "tank_he"], evidenceTick: 90 };
  const p = contactPanel(c, 90 + 5 * 30, RULES);
  expect(p.name).toBe("UNKNOWN");
  expect(p.weapons.map(weaponLabel)).toEqual(["CANNON AP · HE"]);
  expect(p.states.at(-1)?.word).toBe("HEARD 5 s AGO");
  expect(heardWeapons(["hmg", "rifle"], RULES).map(weaponLabel)).toEqual(["HMG", "RIFLE"]);
  expect(contactPanel({ ...c, heard: [] }, 90, RULES).weapons).toEqual([]);
});

const mount = (m: Partial<MountView>): MountView => ({
  mount: 0,
  loaded: 0,
  ammo: [null],
  aim: 1,
  reload: 0,
  target: null,
  reason: "no_compatible_target",
  guiding: false,
  reloading: null,
  ...m,
});

test("every panel is titled with its unit's name, own or enemy, and UNKNOWN for an unknown type", () => {
  const truck = ownPanel(unit({ kind: "supply", mounts: [], stock: 600 }), [], RULES);
  expect(truck.name).toBe("SUPPLY TRUCK");
  expect(ownPanel(unit({ kind: "rifle", mounts: [] }), [], RULES).name).toBe("RIFLE SQUAD");
  expect(enemyPanel("jeep", RULES).name).toBe("JEEP");
  const report = { source: "firing", kind: null, heard: [], evidenceTick: 0 };
  expect(contactPanel(report, 0, RULES).name).toBe("UNKNOWN");
});

test("an own weapon row is the enemy's row with its rounds left: unlimited, a count, each kind of a cannon", () => {
  const squad = unit({
    kind: "rifle",
    mounts: [mount({ mount: 0, ammo: [null] }), mount({ mount: 1, ammo: [0], loaded: null })],
  });
  expect(ownPanel(squad, [], RULES).weapons.map(weaponLabel)).toEqual(["RIFLE ∞", "GRENADE 0"]);
  const tank = unit({
    kind: "tank",
    mounts: [
      mount({ mount: 0, ammo: [20, 15], loaded: null, reloading: 1 }),
      mount({ mount: 1, ammo: [null] }),
    ],
  });
  const rows = ownPanel(tank, [], RULES).weapons;
  expect(rows.map(weaponLabel)).toEqual(["CANNON AP 20 · HE 15", "HMG ∞"]);
  // The kind being loaded is the one marked, and its icon the row's.
  expect(rows[0].kinds.map((k) => k.loaded)).toEqual([false, true]);
  expect(rows[0].icon).toBe("weapons/he_shell.svg");
  // The enemy's rows are the same words without a count or a loaded kind.
  const enemy = enemyPanel("tank", RULES).weapons;
  expect(enemy.map(weaponLabel)).toEqual(["CANNON AP · HE", "HMG"]);
  expect(enemy.flatMap((w) => w.kinds).some((k) => k.loaded || k.count !== undefined)).toBe(false);
  expect(enemy.every((w) => w.live === null)).toBe(true);
});

test("an own weapon row carries its running timers and why it can't fire", () => {
  const t = { kind: "identified" as const, id: 1 };
  const [w] = ownPanel(
    unit({
      kind: "rifle",
      mounts: [mount({ target: t, aim: 0.4, loaded: null, reload: 0.25, reason: "reloading" })],
    }),
    [],
    RULES,
  ).weapons;
  expect(w.live).toMatchObject({ aim: 0.4, reload: 0.25, reason: "reloading" });
  const [out] = ownPanel(
    unit({ kind: "rifle", mounts: [mount({ reason: "out_of_range" })] }),
    [],
    RULES,
  ).weapons;
  expect(out.live).toMatchObject({ aim: null, reload: null, reason: "out_of_range" });
});
