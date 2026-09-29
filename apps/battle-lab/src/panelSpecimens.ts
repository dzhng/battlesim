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
import { UNITS, WEAPONS } from "@packages/scene-assets/src/shippedUnits";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import {
  contactPanel,
  enemyPanel,
  ownPanel,
  type Panel,
  type PanelRules,
} from "@web/battle/present/panelRows";
import { REASON_MARK, type PanelZoom } from "@web/battle/present/infoPanel";
import type { MountView, OwnUnitView } from "@web/battle/sim/observation";

export interface Specimen {
  /** Unique; the scene's key. */
  id: string;
  group: string;
  /** What the specimen shows, as the sheet labels it. */
  label: string;
  owner: "own" | "enemy" | "contact";
  selected: boolean;
  zoom: PanelZoom;
  panel: Panel;
}

type MountPatch = Partial<MountView>;

/** A full, idle mount of `kind`'s mount `k`. */
function fullMount(kind: string, k: number, units: UnitCatalog): MountView {
  const rows = units.type(kind).mounts[k].weapons;
  return {
    mount: k,
    loaded: 0,
    ammo: rows.map((r) => {
      const a = (WEAPONS[r] as { ammo?: number | "unlimited" } | undefined)?.ammo;
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

/** An own unit of `kind`, idle and full, with `patch` over it and each of
 *  `mounts` over its mount of that index. */
function own(
  kind: string,
  patch: Partial<OwnUnitView> = {},
  mounts: MountPatch[] = [],
  units: UnitCatalog = UNITS,
): OwnUnitView {
  const t = units.type(kind);
  return {
    id: 1,
    kind,
    position: [0, 0, 0],
    state: "idle",
    suppression: "none",
    deployment: t.capabilities?.deploy ? { progress: 0, target: "packed" } : null,
    garrison: null,
    stock: t.capabilities?.supply ? t.capabilities.supply.stock : null,
    service: "out_of_range",
    mounts: t.mounts.map((_, k) => ({ ...fullMount(kind, k, units), ...mounts[k] })),
    ...patch,
  } as OwnUnitView;
}

/** Every specimen, in sheet order. */
export function panelSpecimens(rules: PanelRules, units: UnitCatalog = UNITS): Specimen[] {
  const out: Specimen[] = [];
  const kinds = units.ids;
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
    ownPanel(u, [u, ...others], rules, units);

  // The key cases first: a timer's ring beside an idle row's bare icon, and
  // amounts as pips (a partial, an empty, unlimited with none).
  const key = "key cases";
  add(
    key,
    "tank mid-reload, HMG idle",
    panelOf(
      own("tank", {}, [
        { loaded: null, reloading: 0, reload: 0.55, ammo: [13, 15], reason: "reloading" },
      ]),
    ),
  );
  add(
    key,
    "rifle aiming, grenade idle",
    panelOf(own("rifle", {}, [{ target: TARGET, aim: 0.4, reason: "aiming" }])),
  );
  add(
    key,
    "truck deploying 40%, partial supply",
    panelOf(own("supply", { deployment: { progress: 0.4, target: "deployed" }, stock: 250 })),
  );
  add(
    key,
    "grenade at 0",
    panelOf(own("rifle", {}, [{}, { ammo: [0], loaded: null, reason: "out_of_ammo" }])),
  );

  // Every type, as the battle opens: idle and full.
  for (const kind of kinds) add("own, idle", kind, panelOf(own(kind, {}, [], units)));

  // Each weapon situation.
  const w = "own weapons";
  add(
    w,
    "rifle aiming",
    panelOf(own("rifle", {}, [{ target: TARGET, aim: 0.35, reason: "aiming" }])),
  );
  add(
    w,
    "rifle reloading, grenade aiming",
    panelOf(
      own("rifle", {}, [
        { loaded: null, reload: 0.6, reloading: 0, reason: "reloading" },
        { target: TARGET, aim: 0.7, reason: "aiming" },
      ]),
    ),
  );
  add(
    w,
    "aiming and reloading at once",
    panelOf(
      own("rifle", {}, [
        {},
        { target: TARGET, aim: 0.5, loaded: null, reload: 0.3, reloading: 0, reason: "reloading" },
      ]),
    ),
  );
  add(
    w,
    "grenade empty",
    panelOf(own("rifle", {}, [{}, { ammo: [0], loaded: null, reason: "out_of_ammo" }])),
  );
  add(w, "tank AP loaded", panelOf(own("tank", {}, [{ loaded: 0, ammo: [17, 15] }])));
  add(
    w,
    "tank loading HE",
    panelOf(
      own("tank", {}, [
        { loaded: null, reloading: 1, reload: 0.45, ammo: [17, 9], reason: "reloading" },
        { target: TARGET, aim: 0.2, reason: "aiming" },
      ]),
    ),
  );
  add(
    w,
    "tank AP empty, turning",
    panelOf(own("tank", {}, [{ loaded: 1, ammo: [0, 3], reason: "turret_traversing" }])),
  );
  add(
    w,
    "AT guiding",
    panelOf(own("at", {}, [{}, { guiding: true, ammo: [3], reason: "guiding" }])),
  );
  add(w, "jeep firing", panelOf(own("jeep", {}, [{ reason: "firing" }])));

  // Every reason a weapon can't fire, as its warning mark.
  for (const reason of Object.keys(REASON_MARK).filter((r) => REASON_MARK[r]))
    add("why it can't fire", reason, panelOf(own("rifle", {}, [{ reason }, { reason }])));

  // Every state row alone, on the unit that shows it.
  const s = "own states";
  const truckAt = (progress: number, target: "deployed" | "packed", patch = {}) =>
    own("supply", { deployment: { progress, target }, ...patch });
  add(s, "deploying 40%", panelOf(truckAt(0.4, "deployed")));
  add(s, "packing 70%", panelOf(truckAt(0.3, "packed")));
  add(s, "deployed", panelOf(truckAt(1, "deployed")));
  add(s, "stock empty", panelOf(truckAt(1, "deployed", { stock: 0 })));
  const served = own("tank", { id: 2, service: "serving" });
  add(s, "supplying", panelOf(truckAt(1, "deployed", { stock: 412 }), [served]));
  add(s, "resupplying", panelOf(served));
  add(s, "supply full", panelOf(own("rifle", { service: "full" })));
  add(s, "cannot supply", panelOf(own("at", { service: "moving" })));
  const inside = (phase: string, progress: number) => ({
    garrison: { building: 3, phase, progress },
  });
  add(s, "entering 50%", panelOf(own("rifle", inside("entering", 0.5) as Partial<OwnUnitView>)));
  add(s, "no room", panelOf(own("rifle", inside("waiting_for_room", 0) as Partial<OwnUnitView>)));
  add(s, "in building", panelOf(own("rifle", inside("inside", 1) as Partial<OwnUnitView>)));
  add(s, "leaving 70%", panelOf(own("rifle", inside("exiting", 0.7) as Partial<OwnUnitView>)));
  add(s, "suppressed", panelOf(own("rifle", { suppression: "suppressed" })));
  add(s, "pinned", panelOf(own("rifle", { suppression: "pinned" })));
  add(s, "waiting", panelOf(own("tank", { state: "waiting" })));
  add(s, "route blocked", panelOf(own("supply", { state: "route_blocked" })));

  // The longest realistic panels.
  const c = "own, busiest";
  add(
    c,
    "squad in a fight",
    panelOf(
      own(
        "rifle",
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
      own("at", { suppression: "pinned", state: "waiting", service: "moving" }, [
        {},
        { ammo: [1], reason: "no_own_sight" },
      ]),
    ),
  );
  add(
    c,
    "tank waiting, supply blocked",
    panelOf(
      own("tank", { state: "waiting", service: "firing" }, [
        { loaded: null, reloading: 0, reload: 0.8, ammo: [2, 0], reason: "reloading" },
        { reason: "friendly_in_line" },
      ]),
    ),
  );
  add(
    c,
    "truck packing, stock low",
    panelOf(own("supply", { deployment: { progress: 0.8, target: "packed" }, stock: 55 })),
  );

  // Selected beside unselected.
  for (const kind of ["rifle", "tank"]) {
    const p = panelOf(own(kind, {}, [{ target: TARGET, aim: 0.6, reason: "aiming" }]));
    add("selected / not", `${kind} selected`, p, "own", { selected: true });
    add("selected / not", `${kind} unselected`, p);
  }

  // The enemy: every type identified, every type last seen, every heard mix.
  for (const kind of kinds) add("enemy identified", kind, enemyPanel(kind, rules, units), "enemy");
  kinds.forEach((kind, i) =>
    add(
      "last seen",
      `${kind}, ${3 + i * 5} s ago`,
      contactPanel(
        { source: "last_seen", kind, heard: [], evidenceTick: 0 },
        (3 + i * 5) * rules.tick_hz,
        rules,
        units,
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
        { source: "firing", kind: null, heard, evidenceTick: 0 },
        i * rules.tick_hz,
        rules,
        units,
      ),
      "contact",
    ),
  );

  // Far out, where only the selection's panels show.
  const far = "far zoom";
  add(far, "rifle, selected", panelOf(own("rifle", { suppression: "suppressed" })), "own", {
    selected: true,
    zoom: "far",
  });
  add(far, "tank, selected", panelOf(own("tank", {}, [{ ammo: [17, 15] }])), "own", {
    selected: true,
    zoom: "far",
  });
  add(far, "truck, selected", panelOf(truckAt(1, "deployed", { stock: 412 })), "own", {
    selected: true,
    zoom: "far",
  });
  return out;
}
