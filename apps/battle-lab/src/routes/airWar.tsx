import { useMemo, useState, type ReactNode } from "react";
import { useLocation } from "react-router";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { highAir, FACTIONS, type UnitType } from "@packages/scene-assets/src/units";
import type { SessionCatalog } from "@web/battle/catalog/compose";
import { useSessionCatalog } from "@web/battle/catalog/context";
import type { MapDefinition } from "@web/maps/resolve";
import { BattleView } from "../BattleView";
import { LAB_FIXTURES, fixtureMap } from "../fixtures";
import { gameCamera } from "../gameCamera";
import { SavedEncounter, SavedMap, savedScenario } from "../savedMaps";
import { TickStatus } from "../TickStatus";

// The air-war checkpoints (specs/done/air-war), each played on the air test
// map through the production battle view, so its cards, fog, orders and
// sound are the game's. A checkpoint is one or more variants, chosen by
// `?variant=`; each is a saved encounter of the air map (its units and
// scripted orders), a line-up of the catalog's air and air-defence types,
// or an empty skirmish on the same field for paid ingress. The simulation
// decides everything; the scenes (`web/scenes/<fixture>.mjs`) hold each
// checkpoint's observation to its slice's claim.

const SEED = 6;

/** The whole 640 m × 520 m field from the south, every unit in frame. */
const FIELD: Camera3DParams = {
  target: [320, 260, 0],
  distance: 640,
  pitch: 0.95,
  yaw: -Math.PI / 2,
  ...gameCamera.lens,
};

type Side = "blue" | "red";

/** Where a variant's battle comes from. */
type Source =
  | { encounter: string }
  /** Every air and air-defence type on the roster's cards, parked in rows. */
  | { lineup: true }
  /** An empty skirmish on the air field: blue buys its aircraft. */
  | { skirmish: true };

interface Variant {
  label: string;
  source: Source;
  /** The side whose observation is drawn and commanded; blue by default. */
  side?: Side;
}

interface Checkpoint {
  title: string;
  variants: Record<string, Variant>;
  /** A developer panel beside the battle: the fielded types' resolved air
   *  records, or the factions' air-defence matrix. */
  panel?: "records" | "matrix";
}

const CHECKPOINTS: Record<string, Checkpoint> = {
  "air-contracts": {
    title: "Air contracts",
    variants: { contracts: { label: "Records", source: { encounter: "contracts" } } },
    panel: "records",
  },
  "high-air": {
    title: "High air",
    variants: { flight: { label: "Jet over the village", source: { encounter: "high-air" } } },
  },
  "air-defense": {
    title: "Layered air defence",
    variants: {
      helicopter: {
        label: "Helicopters",
        source: { encounter: "defense-helicopter" },
        side: "red",
      },
      jet: { label: "Jet", source: { encounter: "defense-jet" }, side: "red" },
    },
  },
  "air-radar": {
    title: "Radar contacts",
    variants: {
      sead: { label: "SEAD aircraft", source: { encounter: "radar-sead" } },
      fighter: { label: "Fighter only", source: { encounter: "radar-fighter" } },
    },
  },
  dogfight: {
    title: "Dogfight",
    variants: { duel: { label: "Two against two", source: { encounter: "dogfight" } } },
  },
  "air-ingress": {
    title: "Ingress and fuel",
    variants: { skirmish: { label: "Paid aircraft", source: { skirmish: true } } },
  },
  "air-ground": {
    title: "Air to ground",
    variants: { missions: { label: "Bomb, strafe, SEAD", source: { encounter: "ground" } } },
  },
  "air-survivability": {
    title: "Survivability",
    variants: { flares: { label: "Flares or none", source: { encounter: "survivability" } } },
  },
  "air-roster": {
    title: "Air roster",
    variants: { lineup: { label: "Line-up", source: { lineup: true } } },
    panel: "matrix",
  },
  "air-hud": {
    title: "Air HUD",
    variants: { cards: { label: "Cards and contacts", source: { encounter: "hud" } } },
  },
  "air-audio": {
    title: "Air audio",
    variants: { alarm: { label: "Air alarm", source: { encounter: "audio" } } },
  },
  "air-combined": {
    title: "Combined air battle",
    variants: { battle: { label: "Battle", source: { encounter: "combined" } } },
  },
};

/** The empty skirmish of the ingress checkpoint: blue's entry on the west
 *  edge, red's on the east, three objectives across the middle; no
 *  preparation, so a purchase dispatches as soon as blue is ready. */
const INGRESS_SKIRMISH = {
  factions: ["us", "eastern"],
  rules: {
    credits_per_minute: 200,
    starting_minutes: 5,
    preparation_s: 0,
    max_units: 30,
    dispatch_interval_s: 1,
  },
  sites: {
    entries: [
      { side: "blue", center: [10, 260], yaw: 0 },
      { side: "red", center: [630, 260], yaw: Math.PI },
    ],
    objectives: [
      { id: "center", center: [320, 260], radius_m: 40, kind: "junction", counterpart: null },
      { id: "south", center: [320, 80], radius_m: 40, kind: "field", counterpart: "north" },
      { id: "north", center: [320, 440], radius_m: 40, kind: "field", counterpart: "south" },
    ],
  },
};

/** The resolved catalog fields the air checkpoints read, which the shared
 *  `UnitType` leaves untyped. */
interface AirType {
  sensors: { ground_m: number; low_air_m?: number; high_air_m?: number };
  capabilities: {
    air?: {
      fuel_capacity_s: number;
      sead: boolean;
      flares?: { capacity: number; reload_s: number };
      afterburner?: { speed_multiplier: number; fuel_multiplier: number };
      radar?: { emits: boolean; detection_m: number };
    };
    air_defense?: { tier: "short" | "medium" | "long" };
    radar?: { emits: boolean; detection_m: number };
  };
}
const air = (t: UnitType) => t as UnitType & AirType;

/** A type's altitude layer, as the simulation names it. */
const layerOf = (t: UnitType) =>
  "air" in t.mobility ? (highAir(t) ? "high_air" : "low_air") : "ground";

/** The roster's air-defence and fixed-wing types, by faction and role:
 *  short, medium and long air defence, SEAD, and the other fixed-wing. */
const ROLES = ["short", "medium", "long", "sead", "fixed_wing"] as const;
type Role = (typeof ROLES)[number];
function roleOf(t: UnitType): Role | null {
  const caps = air(t).capabilities;
  if (caps.air_defense) return caps.air_defense.tier;
  if (highAir(t)) return caps.air?.sead ? "sead" : "fixed_wing";
  return null;
}
function rosterMatrix(catalog: SessionCatalog) {
  return FACTIONS.map((faction) => ({
    faction,
    roles: Object.fromEntries(
      ROLES.map((role) => [
        role,
        catalog.units.cards.filter(
          (c) =>
            !c.disabled_reason &&
            c.roster.factions.includes(faction) &&
            catalog.units.has(c.id) &&
            roleOf(catalog.units.type(c.id)) === role,
        ),
      ]),
    ) as Record<Role, SessionCatalog["units"]["cards"][number][]>,
  }));
}

/** Every enabled roster type with an air role, faction by faction, parked
 *  in rows of eight across the open field north of the village. */
function lineupEncounter(catalog: SessionCatalog) {
  const kinds = new Set(
    rosterMatrix(catalog).flatMap((row) =>
      ROLES.flatMap((role) => row.roles[role].map((c) => c.id)),
    ),
  );
  return {
    units: [...kinds].map((kind, i) => ({
      side: "blue",
      kind,
      position: [40 + (i % 8) * 75, 40 + Math.floor(i / 8) * 70],
      yaw: Math.PI / 2,
      engagement: "return_fire_only",
    })),
  };
}

export default function AirWar() {
  const { pathname } = useLocation();
  const fixture = LAB_FIXTURES.find((f) => f.route === pathname)?.id;
  const checkpoint = fixture ? CHECKPOINTS[fixture] : undefined;
  if (!fixture || !checkpoint)
    return <main className="lab-rejected">No air checkpoint at {pathname}</main>;
  return <Checkpoint key={fixture} fixture={fixture} checkpoint={checkpoint} />;
}

function Checkpoint({ fixture, checkpoint }: { fixture: string; checkpoint: Checkpoint }) {
  const names = Object.keys(checkpoint.variants);
  const [variant, setVariant] = useState(() => {
    const asked = new URLSearchParams(window.location.search).get("variant");
    return names.find((name) => name === asked) ?? names[0];
  });
  const choose = (next: string) => {
    const url = new URL(window.location.href);
    url.searchParams.set("variant", next);
    window.history.replaceState(window.history.state, "", url);
    setVariant(next);
  };
  const chosen = checkpoint.variants[variant];
  const view = (scenario: string) => (
    <>
      <BattleView
        key={variant}
        fixture={fixture}
        scenario={scenario}
        seed={SEED}
        side={chosen.side ?? "blue"}
        camera={FIELD}
        status={({ sim }) => (
          <>
            <strong>{checkpoint.title}</strong>
            <TickStatus tick={sim.observation?.tick} status={sim.status.status} />
            {names.length > 1 && (
              <span className="lab-row">
                {names.map((name) => (
                  <button
                    key={name}
                    type="button"
                    aria-pressed={name === variant}
                    onClick={() => choose(name)}
                  >
                    {checkpoint.variants[name].label}
                  </button>
                ))}
              </span>
            )}
          </>
        )}
      />
      {checkpoint.panel && <AirPanel kind={checkpoint.panel} scenario={scenario} />}
    </>
  );
  const { source } = chosen;
  if ("encounter" in source)
    return (
      <SavedEncounter key={variant} fixture={fixture} encounter={source.encounter}>
        {(battle) => view(battle.scenario)}
      </SavedEncounter>
    );
  return (
    <SavedMap id={fixtureMap(fixture)}>
      {(map) => (
        <BuiltScenario map={map} source={source}>
          {view}
        </BuiltScenario>
      )}
    </SavedMap>
  );
}

/** A scenario built on the page from the catalog: the roster line-up or the
 *  ingress skirmish, under the session catalog's rules. */
function BuiltScenario({
  map,
  source,
  children,
}: {
  map: MapDefinition;
  source: { lineup: true } | { skirmish: true };
  children: (scenario: string) => ReactNode;
}) {
  const catalog = useSessionCatalog();
  const scenario = useMemo(
    () =>
      "lineup" in source
        ? savedScenario(map, lineupEncounter(catalog), catalog.rules)
        : JSON.stringify({
            map,
            rules: catalog.rules,
            units: [],
            events: [],
            scripts: [],
            skirmish: INGRESS_SKIRMISH,
          }),
    [map, source, catalog],
  );
  return children(scenario);
}

/** The developer panel: what the fielded types resolve to, or the roster's
 *  air-defence matrix. Lab chrome, not the player's HUD. */
function AirPanel({ kind, scenario }: { kind: "records" | "matrix"; scenario: string }) {
  const catalog = useSessionCatalog();
  const fielded = useMemo(
    () => [
      ...new Set((JSON.parse(scenario) as { units: { kind: string }[] }).units.map((u) => u.kind)),
    ],
    [scenario],
  );
  return (
    <aside
      className="hud-panel lab-panel lab-inspector"
      data-occludes-readouts
      data-testid={`air-${kind}`}
      style={{ maxWidth: 460, maxHeight: "60%" }}
    >
      {kind === "records" ? (
        fielded.map((id) => <AirRecord key={id} catalog={catalog} id={id} />)
      ) : (
        <AirMatrix catalog={catalog} />
      )}
    </aside>
  );
}

function AirRecord({ catalog, id }: { catalog: SessionCatalog; id: string }) {
  const t = catalog.units.type(id);
  const { sensors, capabilities: caps } = air(t);
  const radar = caps.air?.radar ?? caps.radar;
  const fields: [string, string][] = [
    ["layer", layerOf(t)],
    ["sensors", `${sensors.ground_m}/${sensors.low_air_m ?? 0}/${sensors.high_air_m ?? 0}`],
    ["fuel", caps.air ? String(caps.air.fuel_capacity_s) : "—"],
    ["flares", caps.air?.flares ? `${caps.air.flares.capacity}/${caps.air.flares.reload_s}` : "—"],
    ["aa_tier", caps.air_defense?.tier ?? "—"],
    ["radar", radar ? `${radar.emits ? "emits" : "silent"}/${radar.detection_m}` : "—"],
    ["sead", caps.air?.sead ? "yes" : "no"],
  ];
  const weapons = t.mounts.flatMap((m) => m.weapons);
  return (
    <section data-kind={id} style={{ marginBottom: 8 }}>
      <strong>{t.name}</strong>
      <div className="lab-row">
        {fields.map(([field, value]) => (
          <span key={field} data-field={field}>
            {field} {value}
          </span>
        ))}
      </div>
      {weapons.map((name) => {
        const w = catalog.weapons[name] as { name: string; range_m: number; [f: string]: unknown };
        const targets = Object.keys((w.targets as object | undefined) ?? { ground: {} }).join("+");
        return (
          <div key={name} data-weapon={name}>
            {w.name}: <span data-field="targets">{targets}</span> {w.range_m} m
            {w.interception ? ` · intercepts ${String(w.interception)}` : ""}
            {w.decoyable ? " · decoyable" : ""}
            {w.seeker ? ` · ${String(w.seeker)} seeker` : ""}
          </div>
        );
      })}
    </section>
  );
}

function AirMatrix({ catalog }: { catalog: SessionCatalog }) {
  return (
    <>
      {rosterMatrix(catalog).map(({ faction, roles }) => (
        <section key={faction} data-faction={faction} style={{ marginBottom: 8 }}>
          <strong>{faction}</strong>
          {ROLES.map((role) => (
            <div key={role} data-role={role}>
              {role.replace("_", " ")}: {roles[role].map((c) => c.name).join(", ") || "—"}
            </div>
          ))}
        </section>
      ))}
    </>
  );
}
