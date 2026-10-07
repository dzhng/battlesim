// @vitest-environment node
// The info panels' rows, derived from the side's observation alone: every
// own unit state as an icon and a short word, and enemy panels that say only
// what the side can know.
import { expect, test, vi } from "vitest";
import {
  contactPanel,
  enemyPanel,
  heardWeapons,
  ownPanel,
  ownStateRows,
  pipsLit,
  unitStrength,
  weaponLabel,
  weaponRows,
  type PanelRules,
} from "../src/battle/present/panelRows";
import type { MountView, OwnUnitView, SuppressionTier } from "../src/battle/sim/observation";
import { UNITS } from "./catalog";

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
  service: { radius_m: 80 },
};

const unit = (u: Partial<OwnUnitView>): OwnUnitView =>
  ({
    id: 1,
    kind: "test_rifle",
    position: [0, 0, 0],
    state: "idle",
    suppression: "none",
    concealed: false,
    deployment: null,
    garrison: null,
    stock: null,
    service: "out_of_range",
    members: [],
    memberHp: [],
    hp: 0,
    ...u,
  }) as OwnUnitView;
const words = (u: OwnUnitView, own: OwnUnitView[] = [u]) =>
  ownStateRows(UNITS, u, own, RULES).map((r) => [r.word, r.progress]);

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

test("only active resupply earns a service row", () => {
  const at = (service: string) => words(unit({ service })).map(([w]) => w);
  expect(at("serving")).toEqual(["RESUPPLYING"]);
  for (const state of [
    "full",
    "moving",
    "firing",
    "no_stock",
    "out_of_range",
    "source_not_deployed",
  ])
    expect(at(state), state).toEqual([]);
});

test("a truck shows its stock, and supplying while set up with a unit in reach being served", () => {
  const truck = unit({
    id: 1,
    kind: "test_supply",
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
  expect(ownStateRows(UNITS, { ...truck, stock: 0 }, [], RULES).map((r) => r.state)).toContain(
    "stock_empty",
  );
});

test("suppression is the published tier's word alone: SUPPRESSED, and PINNED in its own hotter tone", () => {
  const rows = (s: SuppressionTier) =>
    ownStateRows(UNITS, unit({ suppression: s }), [], RULES).map((r) => [
      r.word,
      r.progress,
      r.fill,
      r.tone,
    ]);
  expect(rows("suppressed")).toEqual([["SUPPRESSED", null, null, "warn"]]);
  expect(rows("pinned")).toEqual([["PINNED", null, null, "pinned"]]);
  expect(rows("none")).toEqual([]);
});

test("lasting states come first, then what is happening now", () => {
  const squad = unit({
    garrison: { building: 3, phase: "inside", progress: 1 },
    suppression: "suppressed",
    service: "serving",
  } as Partial<OwnUnitView>);
  expect(words(squad).map(([w]) => w)).toEqual(["IN BUILDING", "SUPPRESSED", "RESUPPLYING"]);
});

test("an amount lights its share of five pips: any at all one, none none", () => {
  expect([0, 0.01, 0.2, 0.5, 0.85, 1].map(pipsLit)).toEqual([0, 1, 1, 3, 4, 5]);
});

test("a truck's stock and a weapon's loaded kind fill against their full amounts; unlimited has none", () => {
  const truck = unit({ kind: "test_supply", stock: 150 });
  const full = UNITS.type("test_supply").capabilities.supply!.stock;
  expect(ownStateRows(UNITS, truck, [], RULES).find((r) => r.state === "stock")?.fill).toBeCloseTo(
    150 / full,
    9,
  );
  const tank = unit({
    kind: "test_tank",
    mounts: [
      mount({ mount: 0, ammo: [17, 5], loaded: null, reloading: 1 }),
      mount({ mount: 1, ammo: [null] }),
    ],
  });
  const [cannon, hmg] = ownPanel(UNITS, tank, [], RULES).weapons;
  // The kind being loaded: HE, 5 of 10.
  expect(cannon.fill).toBe(0.5);
  expect(hmg.fill).toBeNull();
  const empty = ownPanel(
    UNITS,
    unit({ kind: "test_rifle", mounts: [mount({}), mount({ mount: 1, ammo: [0], loaded: null })] }),
    [],
    RULES,
  ).weapons[1];
  expect(empty.fill).toBe(0);
  expect(enemyPanel(UNITS, "test_tank", RULES).weapons.every((w) => w.fill === null)).toBe(true);
});

test("a building's phases and waiting for the way ahead are rows", () => {
  const g = (phase: string, progress: number) =>
    words(unit({ garrison: { building: 3, phase, progress, center: [0, 0], half: [5, 5] } }));
  expect(g("entering", 0.5)).toEqual([["ENTERING", 0.5]]);
  expect(g("inside", 1)).toEqual([["IN BUILDING", null]]);
  expect(g("exiting", 0.25)).toEqual([["LEAVING", 0.25]]);
  expect(words(unit({ state: "waiting" }))).toEqual([["WAITING", null]]);
  // No known route is its own warning, apart from waiting on traffic.
  expect(
    ownStateRows(UNITS, unit({ state: "route_blocked" }), [], RULES).map((r) => [r.word, r.tone]),
  ).toEqual([["ROUTE BLOCKED", "warn"]]);
});

test("strength counts a squad's losses as well as its wounds, and an enemy's is unknown", () => {
  // A full squad at full health, then half of it left at half.
  const hp = UNITS.slots("test_rifle").map((kind) => UNITS.soldier(kind).hp);
  expect(unitStrength(UNITS, unit({ memberHp: hp }))).toBe(1);
  const half = unit({ memberHp: hp.slice(0, hp.length / 2).map((h) => h / 2) });
  expect(unitStrength(UNITS, half)).toBe(0.25);
  // A squad a battle started tougher than its type reads full, never past it.
  expect(unitStrength(UNITS, unit({ memberHp: hp.map((h) => 3 * h) }))).toBe(1);
  const full = UNITS.hull("test_tank")!.hp;
  const tank = unit({ kind: "test_tank", hp: 0.4 * full, mounts: [] });
  // The panel carries it for its name line's pips.
  expect(ownPanel(UNITS, tank, [], RULES).strength).toBeCloseTo(0.4, 9);
  expect(enemyPanel(UNITS, "test_tank", RULES).strength).toBeNull();
});

test("an identified enemy's panel names its type and weapon types, never a count", () => {
  const tank = enemyPanel(UNITS, "test_tank", RULES);
  expect(tank.name).toBe("TANK");
  expect(tank.weapons.map(weaponLabel)).toEqual(["CANNON AP · HE", "HMG"]);
  const squad = enemyPanel(UNITS, "test_rifle", RULES);
  expect(squad.weapons.map(weaponLabel)).toEqual(["RIFLE", "GRENADE"]);
  // Nothing numeric: no ammunition, health or strength.
  for (const p of [tank, squad])
    expect(JSON.stringify([p.name, p.weapons.map(weaponLabel)])).not.toMatch(/\d/);
});

test("a last sighting keeps its identified name and weapons and counts up since", () => {
  const c = { source: "last_seen", kind: "test_tank", heard: [], evidenceTick: 300 };
  const p = contactPanel(UNITS, c, 300 + 12 * 30 + 7, RULES);
  expect(p.name).toBe("TANK");
  expect(p.weapons.map(weaponLabel)).toEqual(["CANNON AP · HE", "HMG"]);
  expect(p.states.at(-1)?.word).toBe("LAST SEEN 12 s AGO");
  // The only number on it is the seconds.
  expect(JSON.stringify([p.name, p.weapons.map(weaponLabel)])).not.toMatch(/\d/);
});

test("a firing report is UNKNOWN, names what was heard by weapon type, and counts up since", () => {
  const c = { source: "firing", kind: null, heard: ["tank_ap", "tank_he"], evidenceTick: 90 };
  const p = contactPanel(UNITS, c, 90 + 5 * 30, RULES);
  expect(p.name).toBe("UNKNOWN");
  expect(p.weapons.map(weaponLabel)).toEqual(["CANNON AP · HE"]);
  expect(p.states.at(-1)?.word).toBe("HEARD 5 s AGO");
  expect(heardWeapons(UNITS, ["hmg", "rifle"], RULES).map(weaponLabel)).toEqual(["HMG", "RIFLE"]);
  expect(contactPanel(UNITS, { ...c, heard: [] }, 90, RULES).weapons).toEqual([]);
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
  const truck = ownPanel(UNITS, unit({ kind: "test_supply", mounts: [], stock: 600 }), [], RULES);
  expect(truck.name).toBe("SUPPLY TRUCK");
  expect(ownPanel(UNITS, unit({ kind: "test_rifle", mounts: [] }), [], RULES).name).toBe(
    "RIFLE SQUAD",
  );
  expect(enemyPanel(UNITS, "test_jeep", RULES).name).toBe("JEEP");
  const report = { source: "firing", kind: null, heard: [], evidenceTick: 0 };
  expect(contactPanel(UNITS, report, 0, RULES).name).toBe("UNKNOWN");
});

test("an own weapon row is the enemy's row with its rounds left: unlimited, a count, each kind of a cannon", () => {
  const squad = unit({
    kind: "test_rifle",
    mounts: [mount({ mount: 0, ammo: [null] }), mount({ mount: 1, ammo: [0], loaded: null })],
  });
  expect(ownPanel(UNITS, squad, [], RULES).weapons.map(weaponLabel)).toEqual([
    "RIFLE ∞",
    "GRENADE 0",
  ]);
  const tank = unit({
    kind: "test_tank",
    mounts: [
      mount({ mount: 0, ammo: [20, 15], loaded: null, reloading: 1 }),
      mount({ mount: 1, ammo: [null] }),
    ],
  });
  const rows = ownPanel(UNITS, tank, [], RULES).weapons;
  expect(rows.map(weaponLabel)).toEqual(["CANNON AP 20 · HE 15", "HMG ∞"]);
  // The kind being loaded is the one marked, and its icon the row's.
  expect(rows[0].kinds.map((k) => k.loaded)).toEqual([false, true]);
  expect(rows[0].icon).toBe("weapons/he_shell.svg");
  // The enemy's rows are the same words without a count or a loaded kind.
  const enemy = enemyPanel(UNITS, "test_tank", RULES).weapons;
  expect(enemy.map(weaponLabel)).toEqual(["CANNON AP · HE", "HMG"]);
  expect(enemy.flatMap((w) => w.kinds).some((k) => k.loaded || k.count !== undefined)).toBe(false);
  expect(enemy.every((w) => w.live === null)).toBe(true);
});

test("physical launchers retain their names, ammo and timers across own, enemy and last-seen panels", () => {
  const at = UNITS.type("test_at");
  const catalog = vi
    .spyOn(UNITS, "type")
    .mockReturnValue({ ...at, mounts: [...at.mounts, at.mounts[1]] });
  try {
    const own = ownPanel(
      UNITS,
      unit({
        kind: "test_at",
        mounts: [
          mount({ mount: 0, ammo: [null] }),
          mount({
            mount: 1,
            ammo: [2],
            loaded: null,
            reloading: 0,
            reload: 0.25,
            reason: "reloading",
          }),
          mount({
            mount: 2,
            ammo: [4],
            loaded: null,
            reloading: 0,
            reload: 0.75,
            reason: "reloading",
          }),
        ],
      }),
      [],
      RULES,
    ).weapons;
    expect(own.map(weaponLabel)).toEqual(["RIFLE ∞", "ATGM 1 2", "ATGM 2 4"]);
    expect(own.slice(1).map((w) => [w.key, w.live?.reload])).toEqual([
      ["1", 0.25],
      ["2", 0.75],
    ]);
    const seen = contactPanel(
      UNITS,
      { source: "last_seen", kind: "test_at", heard: [], evidenceTick: 0 },
      30,
      RULES,
    ).weapons;
    for (const rows of [enemyPanel(UNITS, "test_at", RULES).weapons, seen]) {
      expect(rows.map(weaponLabel)).toEqual(["RIFLE", "ATGM 1", "ATGM 2"]);
      expect(
        rows.every(
          (w) => w.live === null && w.fill === null && w.kinds.every((k) => k.count === undefined),
        ),
      ).toBe(true);
    }
    expect(heardWeapons(UNITS, ["atgm", "atgm"], RULES).map(weaponLabel)).toEqual(["ATGM"]);
  } finally {
    catalog.mockRestore();
  }
});

test("vehicle guns use meaningful mounting names, keep separate timers and retain their rig keys", () => {
  const hmg = UNITS.type("test_tank").mounts[1];
  const mounts = [
    { ...hmg, name: "turret HMG" },
    { ...hmg, name: "hull HMG" },
  ];
  const readiness = [
    mount({ mount: 0, ammo: [null], loaded: null, reloading: 0, reload: 0.4, reason: "reloading" }),
    mount({ mount: 1, ammo: [null] }),
  ];
  const own = weaponRows(mounts, RULES, readiness);
  expect(own.map(weaponLabel)).toEqual(["TURRET HMG ∞", "HULL HMG ∞"]);
  expect(own.map((w) => w.live?.reload)).toEqual([0.4, null]);
  expect(weaponRows(mounts, RULES).map(weaponLabel)).toEqual(["TURRET HMG", "HULL HMG"]);
  expect(mounts.map((m) => m.name)).toEqual(["turret HMG", "hull HMG"]);
});

test("an own weapon row carries its running timers and why it can't fire", () => {
  const t = { kind: "identified" as const, id: 1 };
  const [w] = ownPanel(
    UNITS,
    unit({
      kind: "test_rifle",
      mounts: [mount({ target: t, aim: 0.4, loaded: null, reload: 0.25, reason: "reloading" })],
    }),
    [],
    RULES,
  ).weapons;
  expect(w.live).toMatchObject({ aim: 0.4, reload: 0.25, reason: "reloading" });
  const [out] = ownPanel(
    UNITS,
    unit({ kind: "test_rifle", mounts: [mount({ reason: "out_of_range" })] }),
    [],
    RULES,
  ).weapons;
  expect(out.live).toMatchObject({ aim: null, reload: null, reason: "out_of_range" });
});

test("concealment bonuses earn a HIDDEN row without replacing the building state", () => {
  const hidden = unit({ concealed: true });
  expect(words(hidden)).toEqual([["HIDDEN", null]]);
  expect(words(unit({ concealed: false }))).toEqual([]);
  expect(
    words(
      unit({ concealed: true, garrison: { phase: "inside", progress: 1 } } as Partial<OwnUnitView>),
    ),
  ).toEqual([
    ["IN BUILDING", null],
    ["HIDDEN", null],
  ]);
});

test("fresh firing keeps a previously identified name and labels its evidence honestly", () => {
  const p = contactPanel(
    UNITS,
    { source: "firing", kind: "test_tank", heard: ["tank_he"], evidenceTick: 90 },
    240,
    RULES,
  );
  expect(p.name).toBe("TANK");
  expect(p.states.map((s) => s.word)).toEqual(["HEARD 5 s AGO"]);
  expect(p.weapons.every((w) => w.live === null && w.fill === null)).toBe(true);
});

test("Trophy shares weapon rows with charges and cooldown while enemy readiness stays private", () => {
  const tank = UNITS.type("test_tank");
  const lookup = vi.spyOn(UNITS, "type").mockReturnValue({
    ...tank,
    capabilities: {
      active_protection: {
        capacity: 4,
        cooldown_s: 3,
        standoff_m: 8,
        service_s: 10,
        stock_per_charge: 20,
      },
    },
  });
  try {
    const own = unit({ kind: "test_tank", mounts: [], protection: { charges: 2, cooldown: 0.5 } });
    const ready = ownPanel(UNITS, own, [own], RULES).weapons.find((row) => row.name === "TROPHY");
    expect(ready?.kinds).toEqual([{ label: null, count: 2, loaded: true }]);
    expect(ready?.fill).toBe(0.5);
    expect(ready?.live?.cooldown).toBe(0.5);
    const enemy = enemyPanel(UNITS, "test_tank", RULES).weapons.find(
      (row) => row.name === "TROPHY",
    );
    expect(enemy?.kinds).toEqual([{ label: null, loaded: false }]);
    expect(enemy?.live).toBeNull();
    expect(enemy?.fill).toBeNull();
  } finally {
    lookup.mockRestore();
  }
});
