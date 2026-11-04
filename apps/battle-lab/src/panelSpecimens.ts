// The panel workbench's specimens: every info panel the battle can draw,
// built from synthetic observations through the battle's own derivation
// (`panelRows.ts`), so the workbench shows what the game shows. The groups
// walk the row sources: every unit type as own, enemy and last sighting;
// each weapon situation (unlimited, counts, empty, aiming, reloading, the
// cannon's loaded kind, guiding, every reason a weapon can't fire); every
// state row alone and in the longest realistic combinations; carriers
// empty, partly full and full, with their riders' cards under them; every
// heard weapon mix; selected and not; the default and the far zoom. A test
// (`web/tests/panelWorkbench.test.ts`) holds this list to every row kind the
// derivation can produce.
import type { SessionCatalog } from "@web/battle/catalog/compose";
import {
  arrivingUnit,
  contactPanel,
  enemyPanel,
  ownPanel,
  weaponRows,
  type Panel,
  type PanelRules,
} from "@web/battle/present/panelRows";
import {
  REASON_MARK,
  type PanelOwner,
  type PanelZoom,
  type PassengerCard,
} from "@web/battle/present/infoPanel";
import { passengersOf, type MountView, type OwnUnitView } from "@web/battle/sim/observationView";
import { UnitCatalog } from "@packages/scene-assets/src/units";

export interface Specimen {
  /** Unique; the scene's key. */
  id: string;
  group: string;
  /** What the specimen shows, as the sheet labels it. */
  label: string;
  owner: PanelOwner;
  selected: boolean;
  zoom: PanelZoom;
  panel: Panel;
  /** A carrier's riders, their compact cards under its panel. */
  passengers?: PassengerCard[];
}

type MountPatch = Partial<MountView>;

const TARGET = { kind: "identified" as const, id: 99 };

/** An own unit of `kind`, idle, full and at full strength, with `patch`
 *  over it and each of `mounts` over its mount of that index. */
export function specimenUnit(
  catalog: SessionCatalog,
  kind: string,
  patch: Partial<OwnUnitView> = {},
  mounts: MountPatch[] = [],
): OwnUnitView {
  const unit = arrivingUnit(catalog, kind);
  return {
    ...unit,
    mounts: unit.mounts.map((m, k) => ({ ...m, ...mounts[k] })),
    ...patch,
  };
}

/** The test units a visual baseline pins, named rather than matched by
 *  prefix: tests add test units of their own, and those must not move the
 *  approved pictures. */
const PINNED_UNITS = new Set([
  "test_rifle",
  "test_recon",
  "test_at",
  "test_tank",
  "test_supply",
  "test_jeep",
  "test_gun_jeep",
  "test_heli",
]);

/** Every specimen, in sheet order. The per-type groups draw every unit type
 *  of the catalog and the heard group every weapon row; with `testUnitsOnly`
 *  (what a visual baseline pins: the roster grows, these do not) they draw
 *  only the pinned test units and the weapons those units carry. */
export function panelSpecimens(
  catalog: SessionCatalog,
  rules: PanelRules,
  { testUnitsOnly = false }: { testUnitsOnly?: boolean } = {},
): Specimen[] {
  const out: Specimen[] = [];
  const kinds = catalog.units.ids.filter((kind) => !testUnitsOnly || PINNED_UNITS.has(kind));
  const carried = new Set(
    kinds.flatMap((kind) => catalog.units.type(kind).mounts.flatMap((m) => m.weapons)),
  );
  const add = (
    group: string,
    label: string,
    panel: Panel,
    owner: Specimen["owner"] = "own",
    extra: Partial<Pick<Specimen, "selected" | "zoom" | "passengers">> = {},
  ) =>
    out.push({
      id: `${group}/${label}`,
      group,
      label,
      owner,
      selected: false,
      zoom: "default",
      panel,
      ...extra,
    });
  const panelOf = (u: OwnUnitView, others: OwnUnitView[] = []) =>
    ownPanel(catalog.units, u, [u, ...others], rules);

  add(
    "withdrawal",
    "returning to base",
    panelOf(specimenUnit(catalog, "test_tank", { withdrawing: true })),
  );
  // The Trophy part's protection, as its catalog row gives it.
  const trophy = { name: "Trophy", icon: "trophy", capacity: 4 };
  for (const [label, charges, cooldown, service] of [
    ["ready", 4, null, "out_of_range"],
    ["cooling", 2, 0.5, "out_of_range"],
    ["empty", 0, null, "out_of_range"],
    ["resupplying", 1, null, "serving"],
  ] as const) {
    const unit = specimenUnit(catalog, "test_tank", { protection: { charges, cooldown }, service });
    const panel = panelOf(unit);
    panel.weapons = weaponRows(
      catalog.units.type(unit.kind).mounts,
      rules,
      unit.mounts,
      trophy,
      unit.protection,
    );
    add("Trophy", label, panel);
  }
  const enemyTrophy = enemyPanel(catalog.units, "test_tank", rules);
  enemyTrophy.weapons = weaponRows(
    catalog.units.type("test_tank").mounts,
    rules,
    undefined,
    trophy,
  );
  add("Trophy", "enemy equipment only", enemyTrophy, "enemy");

  // The key cases first: a timer's ring beside an idle row's bare icon, and
  // amounts as pips (a partial, an empty, unlimited with none).
  const key = "key cases";
  add(
    key,
    "tank mid-reload, HMG idle",
    panelOf(
      specimenUnit(catalog, "test_tank", {}, [
        { loaded: null, reloading: 0, reload: 0.55, ammo: [13, 15], reason: "reloading" },
      ]),
    ),
  );
  add(
    key,
    "rifle aiming, grenade idle",
    panelOf(
      specimenUnit(catalog, "test_rifle", {}, [{ target: TARGET, aim: 0.4, reason: "aiming" }]),
    ),
  );
  add(
    key,
    "truck deploying 40%, partial supply",
    panelOf(
      specimenUnit(catalog, "test_supply", {
        deployment: { progress: 0.4, target: "deployed" },
        stock: 250,
      }),
    ),
  );
  add(
    key,
    "grenade at 0",
    panelOf(
      specimenUnit(catalog, "test_rifle", {}, [
        {},
        { ammo: [0], loaded: null, reason: "out_of_ammo" },
      ]),
    ),
  );
  const alertPanel = panelOf(
    specimenUnit(catalog, "test_rifle", { concealed: true, suppression: "pinned" }, [
      {},
      { ammo: [0], loaded: null, reason: "out_of_ammo" },
    ]),
  );
  add(key, "hidden + pinned + out of ammo", alertPanel);
  add(key, "hidden + pinned + out of ammo, compact", alertPanel, "own", {
    zoom: "compressed",
  });
  add(
    key,
    "compact: deployment and reload",
    panelOf(
      specimenUnit(catalog, "test_supply", { deployment: { progress: 0.35, target: "deployed" } }, [
        { loaded: null, reload: 0.8, reloading: 0, reason: "reloading" },
      ]),
    ),
    "own",
    { zoom: "compressed" },
  );
  add(
    key,
    "compact: aiming and reload",
    panelOf(
      specimenUnit(catalog, "test_rifle", {}, [
        {},
        {
          target: TARGET,
          aim: 0.45,
          loaded: null,
          reload: 0.85,
          reloading: 0,
          reason: "reloading",
        },
      ]),
    ),
    "own",
    { zoom: "compressed" },
  );

  // Synthetic equipment configurations, through the production row builder.
  // Keep them here rather than retuning a playable unit just for a UI specimen.
  const atMounts = catalog.units.type("test_at").mounts;
  const paired = [...atMounts, atMounts[1]];
  const pairedReadiness = specimenUnit(catalog, "test_at").mounts.concat({
    ...arrivingUnit(catalog, "test_at").mounts[1],
    mount: 2,
  });
  pairedReadiness[1] = {
    ...pairedReadiness[1],
    ammo: [2],
    loaded: null,
    reloading: 0,
    reload: 0.25,
    reason: "reloading",
  };
  pairedReadiness[2] = {
    ...pairedReadiness[2],
    ammo: [3],
    loaded: null,
    reloading: 0,
    reload: 0.75,
    reason: "reloading",
  };
  const twinLaunchers = {
    ...panelOf(specimenUnit(catalog, "test_at")),
    weapons: weaponRows(paired, rules, pairedReadiness),
  };
  add(key, "two launchers, separate reloads", twinLaunchers, "own", { selected: true });
  add(
    key,
    "two launchers, enemy equipment",
    { ...enemyPanel(catalog.units, "test_at", rules), weapons: weaponRows(paired, rules) },
    "enemy",
  );
  const hmg = catalog.units.type("test_tank").mounts[1];
  const twinHmg = [
    { ...hmg, name: "turret HMG" },
    { ...hmg, name: "hull HMG" },
  ];
  add(key, "turret and hull HMG", {
    ...panelOf(specimenUnit(catalog, "test_tank")),
    weapons: weaponRows(twinHmg, rules, [
      {
        ...arrivingUnit(catalog, "test_tank").mounts[1],
        mount: 0,
        loaded: null,
        reloading: 0,
        reload: 0.4,
        reason: "reloading",
      },
      { ...arrivingUnit(catalog, "test_tank").mounts[1], mount: 1 },
    ]),
  });
  add("far out", "two launchers selected", twinLaunchers, "own", { selected: true, zoom: "far" });

  // Every type, as the battle opens: idle and full.
  for (const kind of kinds) add("own, idle", kind, panelOf(specimenUnit(catalog, kind, {}, [])));

  // Each weapon situation (the key cases' aside).
  const w = "own weapons";
  add(
    w,
    "rifle reloading, grenade aiming",
    panelOf(
      specimenUnit(catalog, "test_rifle", {}, [
        { loaded: null, reload: 0.6, reloading: 0, reason: "reloading" },
        { target: TARGET, aim: 0.7, reason: "aiming" },
      ]),
    ),
  );
  add(
    w,
    "aiming and reloading at once",
    panelOf(
      specimenUnit(catalog, "test_rifle", {}, [
        {},
        { target: TARGET, aim: 0.5, loaded: null, reload: 0.3, reloading: 0, reason: "reloading" },
      ]),
    ),
  );
  add(
    w,
    "aim complete, reload continues",
    panelOf(
      specimenUnit(catalog, "test_rifle", {}, [
        {},
        { target: TARGET, aim: 1, loaded: null, reload: 0.3, reloading: 0, reason: "reloading" },
      ]),
    ),
  );
  add(
    w,
    "tank AP loaded",
    panelOf(specimenUnit(catalog, "test_tank", {}, [{ loaded: 0, ammo: [17, 15] }])),
  );
  add(
    w,
    "tank loading HE",
    panelOf(
      specimenUnit(catalog, "test_tank", {}, [
        { loaded: null, reloading: 1, reload: 0.45, ammo: [17, 9], reason: "reloading" },
        { target: TARGET, aim: 0.2, reason: "aiming" },
      ]),
    ),
  );
  add(
    w,
    "tank AP empty, turning",
    panelOf(
      specimenUnit(catalog, "test_tank", {}, [
        { loaded: 1, ammo: [0, 3], reason: "turret_traversing" },
      ]),
    ),
  );
  add(
    w,
    "AT guiding",
    panelOf(
      specimenUnit(catalog, "test_at", {}, [{}, { guiding: true, ammo: [3], reason: "guiding" }]),
    ),
  );
  add(w, "jeep firing", panelOf(specimenUnit(catalog, "test_jeep", {}, [{ reason: "firing" }])));

  // Every reason a weapon can't fire, as its warning mark.
  for (const reason of Object.keys(REASON_MARK).filter((r) => REASON_MARK[r]))
    add(
      "why it can't fire",
      reason,
      panelOf(specimenUnit(catalog, "test_rifle", {}, [{ reason }, { reason }])),
    );

  // Every state row alone, on the unit that shows it.
  const s = "own states";
  const truckAt = (progress: number, target: "deployed" | "packed", patch = {}) =>
    specimenUnit(catalog, "test_supply", { deployment: { progress, target }, ...patch });
  add(s, "deploying 40%", panelOf(truckAt(0.4, "deployed")));
  add(s, "packing 70%", panelOf(truckAt(0.3, "packed")));
  add(s, "deployed", panelOf(truckAt(1, "deployed")));
  add(s, "stock empty", panelOf(truckAt(1, "deployed", { stock: 0 })));
  const served = specimenUnit(catalog, "test_tank", { id: 2, service: "serving" });
  add(s, "supplying", panelOf(truckAt(1, "deployed", { stock: 412 }), [served]));
  add(s, "resupplying", panelOf(served));
  add(s, "supply full", panelOf(specimenUnit(catalog, "test_rifle", { service: "full" })));
  add(s, "cannot supply", panelOf(specimenUnit(catalog, "test_at", { service: "moving" })));
  const inside = (phase: string, progress: number) => ({
    hold: { host: { kind: "building", id: 3, center: [0, 0], half: [10, 10] }, phase, progress },
    concealed: phase === "inside" || phase === "exiting",
  });
  add(
    s,
    "entering 50%",
    panelOf(specimenUnit(catalog, "test_rifle", inside("entering", 0.5) as Partial<OwnUnitView>)),
  );
  add(s, "hidden in foliage", panelOf(specimenUnit(catalog, "test_rifle", { concealed: true })));
  add(
    s,
    "hidden, compressed",
    panelOf(specimenUnit(catalog, "test_rifle", { concealed: true })),
    "own",
    {
      zoom: "compressed",
    },
  );
  add(
    s,
    "in building",
    panelOf(specimenUnit(catalog, "test_rifle", inside("inside", 1) as Partial<OwnUnitView>)),
  );
  add(
    s,
    "leaving 70%",
    panelOf(specimenUnit(catalog, "test_rifle", inside("exiting", 0.7) as Partial<OwnUnitView>)),
  );
  add(s, "suppressed", panelOf(specimenUnit(catalog, "test_rifle", { suppression: "suppressed" })));
  add(s, "pinned", panelOf(specimenUnit(catalog, "test_rifle", { suppression: "pinned" })));
  add(s, "waiting", panelOf(specimenUnit(catalog, "test_tank", { state: "waiting" })));
  add(
    s,
    "route blocked",
    panelOf(specimenUnit(catalog, "test_supply", { state: "route_blocked" })),
  );

  // Carriers: the test APC (8 seats), given 16 for the largest, and the
  // test scout car. The squads it holds are stand-ins of set sizes, so the
  // counts read the same whatever the roster's squads number.
  const t = "carriers";
  const sixteen = new UnitCatalog({
    ...catalog.units.view,
    units: catalog.units.view.units.map((u) =>
      u.id === "test_apc"
        ? { ...u, capabilities: { ...u.capabilities, carriage: { seats: 16 } } }
        : u,
    ),
  });
  const carrier = specimenUnit(catalog, "test_apc", { id: 1 });
  const rider = (id: number, kind: string, soldiers: number, patch: Partial<OwnUnitView> = {}) =>
    specimenUnit(catalog, kind, {
      id,
      memberHp: Array.from({ length: soldiers }, () => 100),
      hold: { host: { kind: "carrier", id: 1 }, phase: "inside", progress: 1 },
      ...patch,
    });
  /** The carrier's panel and its riders' cards, from one side's own units. */
  const carrying = (
    label: string,
    riders: OwnUnitView[],
    extra: Partial<Pick<Specimen, "selected" | "zoom">> = {},
    units = catalog.units,
  ) => {
    const own = [carrier, ...riders];
    add(t, label, ownPanel(units, carrier, own, rules), "own", {
      ...extra,
      passengers: passengersOf(own, carrier.id).map((p) => ({
        id: p.id,
        panel: ownPanel(units, p, own, rules),
      })),
    });
  };
  const partly = [rider(2, "test_at", 3)];
  const full = [
    rider(2, "test_rifle", 8, { memberHp: [100, 100, 100, 100, 100, 100, 40, 100] }),
    rider(3, "test_at", 3),
    rider(4, "test_recon", 3),
    rider(5, "test_rifle", 2, { suppression: "suppressed" }),
  ];
  carrying("empty, 8 seats", []);
  carrying("empty, compact", [], { zoom: "compressed" });
  carrying("partly full", partly);
  carrying("partly full, compact", partly, { zoom: "compressed" });
  carrying("full, 16 seats", full, { selected: true }, sixteen);
  carrying("full, 16 seats, compact", full, { zoom: "compressed" }, sixteen);
  add(t, "enemy carrier", enemyPanel(catalog.units, "test_scout_car", rules), "enemy");
  // A squad getting in or out has a timer, and its own row.
  for (const [label, phase, progress] of [
    ["squad boarding 60%", "entering", 0.6],
    ["squad dismounting 30%", "exiting", 0.3],
  ] as const) {
    const u = rider(2, "test_rifle", 8, {
      hold: { host: { kind: "carrier", id: 1 }, phase, progress },
    });
    add(t, label, ownPanel(catalog.units, u, [carrier, u], rules));
  }

  // An aircraft's rows. They come from the published air state alone, never
  // the kind, and no stand-in flies, so they ride the jeep.
  const a = "aircraft states";
  const flying = (patch: Partial<OwnUnitView>) =>
    panelOf(specimenUnit(catalog, "test_jeep", { fuelS: 142.4, ...patch }));
  add(a, "fuel 143 s", flying({}));
  add(a, "fuel out", flying({ fuelS: 0 }));
  add(a, "afterburners on", flying({ afterburner: true }));
  add(a, "dogfight", flying({ inDogfight: true }));
  add(a, "flares ready", flying({ flares: { charges: 4, cooldown: null } }));
  add(a, "flares reloading", flying({ flares: { charges: 1, cooldown: 0.6 } }));
  add(
    a,
    "dogfight on afterburner, flares out",
    flying({
      fuelS: 37.8,
      afterburner: true,
      inDogfight: true,
      flares: { charges: 0, cooldown: 0.25 },
    }),
  );
  // A type that carries a radar receiver says so, as a lasting row; no
  // stand-in carries one, so the jeep's type is given one here.
  const view = catalog.units.view;
  const receiving = new UnitCatalog({
    ...view,
    units: view.units.map((t) =>
      t.id === "test_jeep"
        ? { ...t, capabilities: { ...t.capabilities, radar_receiver: { range_m: 5000 } } }
        : t,
    ),
  });
  const receiver = specimenUnit(catalog, "test_jeep");
  add(a, "finds radars", ownPanel(receiving, receiver, [receiver], rules));
  // What a radar receiver shows of an emitter: the warning
  // alone, and with the bearing and range it measured.
  const radar = (bearingDeg: number | null, rangeM: number | null) =>
    contactPanel(
      catalog.units,
      { source: "radar", kind: null, heard: [], evidenceTick: 0, bearingDeg, rangeM },
      0,
      rules,
    );
  add(a, "radar contact", radar(null, null), "contact");
  add(a, "radar contact, bearing and range", radar(47, 6350), "contact");

  // The longest realistic panels.
  const c = "own, busiest";
  add(
    c,
    "squad in a fight",
    panelOf(
      specimenUnit(
        catalog,
        "test_rifle",
        {
          ...(inside("inside", 1) as Partial<OwnUnitView>),
          suppression: "suppressed",
          service: "serving",
        },
        [
          { target: TARGET, aim: 0.5, reason: "aiming" },
          { loaded: null, reload: 0.3, reloading: 0, ammo: [2], reason: "reloading" },
        ],
      ),
    ),
  );
  add(
    c,
    "AT team pinned",
    panelOf(
      specimenUnit(
        catalog,
        "test_at",
        { suppression: "pinned", state: "waiting", service: "moving" },
        [{}, { ammo: [1], reason: "no_own_sight" }],
      ),
    ),
  );
  add(
    c,
    "tank waiting, supply blocked",
    panelOf(
      specimenUnit(catalog, "test_tank", { state: "waiting", service: "firing" }, [
        { loaded: null, reloading: 0, reload: 0.8, ammo: [2, 0], reason: "reloading" },
        { reason: "friendly_in_line" },
      ]),
    ),
  );
  add(
    c,
    "truck packing, stock low",
    panelOf(
      specimenUnit(catalog, "test_supply", {
        deployment: { progress: 0.8, target: "packed" },
        stock: 55,
      }),
    ),
  );

  // Selected beside unselected.
  for (const kind of ["test_rifle", "test_tank"]) {
    const p = panelOf(
      specimenUnit(catalog, kind, {}, [{ target: TARGET, aim: 0.6, reason: "aiming" }]),
    );
    add("selected / not", `${kind} selected`, p, "own", { selected: true });
    add("selected / not", `${kind} unselected`, p);
  }

  // The enemy: every type identified, every type last seen, every heard mix.
  for (const kind of kinds)
    add("enemy identified", kind, enemyPanel(catalog.units, kind, rules), "enemy");
  kinds.forEach((kind, i) =>
    add(
      "last seen",
      `${kind}, ${3 + i * 5} s ago`,
      contactPanel(
        catalog.units,
        { source: "last_seen", kind, heard: [], evidenceTick: 0 },
        (3 + i * 5) * rules.tick_hz,
        rules,
      ),
      "contact",
    ),
  );
  const rows = testUnitsOnly
    ? ["rifle", "grenade", "tank_ap", "tank_he", "hmg", "atgm", "autocannon"].filter((r) =>
        carried.has(r),
      )
    : Object.keys(rules.weapons);
  const mixes = [
    [],
    ...rows.map((r) => [r]),
    ["rifle", "grenade"],
    ["tank_ap", "tank_he", "hmg"],
    ["rifle", "atgm"],
  ].filter((m) => m.every((r) => rows.includes(r)));
  mixes.forEach((heard, i) =>
    add(
      "heard",
      heard.length ? heard.join(" + ") : "nothing named",
      contactPanel(
        catalog.units,
        { source: "firing", kind: null, heard, evidenceTick: 0 },
        i * rules.tick_hz,
        rules,
      ),
      "contact",
    ),
  );

  // Far out, where only the selection's panels show.
  const far = "far zoom";
  add(
    far,
    "rifle, selected",
    panelOf(specimenUnit(catalog, "test_rifle", { suppression: "suppressed" })),
    "own",
    {
      selected: true,
      zoom: "far",
    },
  );
  add(
    far,
    "tank, selected",
    panelOf(specimenUnit(catalog, "test_tank", {}, [{ ammo: [17, 15] }])),
    "own",
    {
      selected: true,
      zoom: "far",
    },
  );
  add(far, "truck, selected", panelOf(truckAt(1, "deployed", { stock: 412 })), "own", {
    selected: true,
    zoom: "far",
  });

  // An ally's units: your own panel, in the ally's tone, naming its owner.
  const allyOf = (u: OwnUnitView) => ({ ...panelOf(u), owner: "BLUE 2 · AI" });
  add("ally", "tank", allyOf(specimenUnit(catalog, "test_tank")), "ally");
  add("ally", "rifle, selected", allyOf(specimenUnit(catalog, "test_rifle")), "ally", {
    selected: true,
  });
  return out;
}
