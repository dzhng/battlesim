// The panel workbench's specimens: every info panel the battle can draw,
// built from synthetic observations through the battle's own derivation
// (`panelRows.ts`), so the workbench shows what the game shows. The groups
// walk the row sources: every unit type as own, enemy and last sighting;
// each weapon situation (unlimited, counts, empty, aiming, reloading, the
// cannon's loaded kind, guiding, every reason a weapon can't fire); every
// state row alone and in the longest realistic combinations; every heard
// weapon mix; selected and not; the default and the far zoom. A test
// (`web/tests/panelWorkbench.test.ts`) holds this list to every row kind the
// derivation can produce.
import type { SessionCatalog } from "@web/battle/catalog/compose";
import {
  contactPanel,
  enemyPanel,
  ownPanel,
  weaponRows,
  type Panel,
  type PanelRules,
} from "@web/battle/present/panelRows";
import { REASON_MARK, type PanelOwner, type PanelZoom } from "@web/battle/present/infoPanel";
import type { MountView, OwnUnitView } from "@web/battle/sim/observation";

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
}

type MountPatch = Partial<MountView>;

/** A full, idle mount of `kind`'s mount `k`. */
function fullMount(catalog: SessionCatalog, kind: string, k: number): MountView {
  const rows = catalog.units.type(kind).mounts[k].weapons;
  return {
    mount: k,
    loaded: 0,
    ammo: rows.map((r) => {
      const a = (catalog.weapons[r] as { ammo?: number | "unlimited" } | undefined)?.ammo;
      return typeof a === "number" ? a : null;
    }),
    aim: 1,
    reload: 0,
    target: null,
    reason: "no_compatible_target",
    guiding: false,
    reloading: null,
  };
}

const TARGET = { kind: "identified" as const, id: 99 };

/** An own unit of `kind`, idle, full and at full strength, with `patch`
 *  over it and each of `mounts` over its mount of that index. */
export function specimenUnit(
  catalog: SessionCatalog,
  kind: string,
  patch: Partial<OwnUnitView> = {},
  mounts: MountPatch[] = [],
): OwnUnitView {
  const t = catalog.units.type(kind);
  return {
    id: 1,
    kind,
    position: [0, 0, 0],
    state: "idle",
    suppression: "none",
    concealed: false,
    deployment: t.capabilities?.deploy ? { progress: 0, target: "packed" } : null,
    garrison: null,
    stock: t.capabilities?.supply ? t.capabilities.supply.stock : null,
    service: "out_of_range",
    hp: catalog.units.hull(kind)?.hp ?? 0,
    memberHp: catalog.units.hull(kind)
      ? []
      : catalog.units.slots(kind).map((s) => catalog.units.soldier(s).hp),
    mounts: t.mounts.map((_, k) => ({ ...fullMount(catalog, kind, k), ...mounts[k] })),
    ...patch,
  } as OwnUnitView;
}

/** Every specimen, in sheet order. */
export function panelSpecimens(catalog: SessionCatalog, rules: PanelRules): Specimen[] {
  const out: Specimen[] = [];
  const kinds = catalog.units.ids;
  const add = (
    group: string,
    label: string,
    panel: Panel,
    owner: Specimen["owner"] = "own",
    extra: Partial<Pick<Specimen, "selected" | "zoom">> = {},
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
    ...fullMount(catalog, "test_at", 1),
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
        ...fullMount(catalog, "test_tank", 1),
        mount: 0,
        loaded: null,
        reloading: 0,
        reload: 0.4,
        reason: "reloading",
      },
      { ...fullMount(catalog, "test_tank", 1), mount: 1 },
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
    garrison: { building: 3, phase, progress },
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
  const rows = Object.keys(rules.weapons);
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
  return out;
}
