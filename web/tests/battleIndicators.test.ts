// @vitest-environment node
// The battle view's indicators (and the capture that films them) follow the
// side's information and nothing more: an own unit and an identified enemy
// carry no contact glyph; a contact, seen, heard or remembered while its
// presentation holds it, carries its hatched report, never an identified
// unit's marks or its type; an enemy the side has not found leaves no mark.
// Each unit stands far from the others, so a mark is found by where it lies.
import { expect, test } from "vitest";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh";
import type { WorldMeshes } from "@packages/battle-renderer/src/scene";
import { screenAxes } from "@packages/renderer-core/src/camera3d";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import { ContactPresentation } from "@web/battle/present/contactPresentation";
import { NOTHING_REVEALED } from "@web/battle/present/orderReveal";
import type { ContactView, ObservationView } from "@web/battle/sim/observationView";
import { buildBattleOverlay, contactLayer } from "@apps/battle-lab/src/battleOverlay";
import { gameContactStyle } from "@apps/battle-lab/src/gameFog";
import { gameStroke } from "@apps/battle-lab/src/gameOverlay";

type P3 = [number, number, number];
const flat = () => 0;
const TICK_HZ = 30;
const HELICOPTER = "test_helicopter";
const units = {
  type: (kind: string) => ({
    mobility:
      kind === HELICOPTER
        ? {
            air: {
              layer: "low_air",
              flight: "rotor",
              cruise_kmh: 200,
              turn_deg_s: 30,
              climb_mps: 8,
            },
          }
        : {},
  }),
  hull: () => ({ half_extents_m: [6, 2, 1.5] }),
} as unknown as UnitCatalog;
const rules = { air: {}, tick_hz: TICK_HZ } as unknown as Parameters<typeof buildBattleOverlay>[1];
const facing = {
  ...screenAxes({ yaw: 0.3, pitch: 0.4 }),
  stroke: gameStroke(0.05),
  size: gameStroke(0.05),
};

// Where the battle's units truly stand, kilometres apart.
const FRIEND: P3 = [0, 0, 0];
const IDENTIFIED: P3 = [1000, 0, 0];
const CONTACT: P3 = [2000, 0, 0];
const HIDDEN: P3 = [3000, 0, 0];
const REMEMBERED: P3 = [4000, 0, 0];
const IDENTIFIED_AIR: P3 = [5000, 0, 80];
const CONTACT_AIR: P3 = [6000, 0, 80];

const report = (id: number, center: P3, aloft = false): ContactView => ({
  id,
  source: "sighting",
  center,
  layer: aloft ? "low_air" : "ground",
  aloft,
  radius: 40,
  evidenceTick: 0,
  expiresTick: 9000,
  kind: null,
  heard: [],
});

/** The side's observation: its own rifle squad, an identified tank and an
 *  identified helicopter, and `contacts`. The hidden enemy is in none of
 *  them, as the authority leaves it out. */
const observation = (tick: number, contacts: ContactView[]) =>
  ({
    tick,
    own: [{ id: 1, kind: "test_rifle", position: FRIEND, yaw: 0, hold: null, stock: null }],
    identified: [
      { id: 2, kind: "test_tank", position: IDENTIFIED, yaw: 0 },
      { id: 6, kind: HELICOPTER, position: IDENTIFIED_AIR, yaw: 0 },
    ],
    contacts,
    encounter: null,
    skirmish: null,
    projectiles: [],
    guided: [],
  }) as unknown as ObservationView;

/** The overlay the battle view draws for `o` with its presented contacts. */
function indicators(o: ObservationView, presentation: ContactPresentation) {
  const contacts = presentation.update(o.contacts, o.tick);
  return buildBattleOverlay(
    units,
    rules,
    o,
    [],
    flat,
    { supplyRadius: 170, zone: null, deployments: [] },
    { showOrders: false, reveal: NOTHING_REVEALED, contacts, facing },
  );
}

/** Whether `channels` of `m` put any vertex within 150 m of `at` on the map. */
function marked(
  m: WorldMeshes,
  at: P3,
  channels: (keyof WorldMeshes)[] = ["opaque", "translucent", "painted", "paintedMarching"],
) {
  return channels.some((c) => {
    const mesh = m[c] ?? new Float32Array(0);
    for (let i = 0; i < mesh.length; i += VERTEX_FLOATS)
      if (Math.hypot(mesh[i] - at[0], mesh[i + 1] - at[1]) < 150) return true;
    return false;
  });
}

const contacts = [report(3, CONTACT), report(7, CONTACT_AIR, true)];

test("own units and identified enemies carry no contact glyph", () => {
  const m = indicators(observation(0, contacts), new ContactPresentation(TICK_HZ, 3));
  expect(marked(m, FRIEND), "an own unit nobody selected").toBe(false);
  expect(marked(m, IDENTIFIED), "an identified ground enemy is its model alone").toBe(false);
  expect(marked(m, IDENTIFIED_AIR, ["translucent"]), "an identified aircraft has no glyph").toBe(
    false,
  );
  expect(marked(m, IDENTIFIED_AIR, ["painted"]), "but its ground marker").toBe(true);
});

test("a contact carries its glyph, and never an identified unit's marks", () => {
  const m = indicators(observation(0, contacts), new ContactPresentation(TICK_HZ, 3));
  expect(marked(m, CONTACT, ["translucent"]), "a ground contact's glyph").toBe(true);
  expect(marked(m, CONTACT_AIR, ["translucent"]), "an aloft contact's sign").toBe(true);
  expect(marked(m, CONTACT_AIR, ["painted", "opaque"]), "no aircraft marker or drop line").toBe(
    false,
  );
});

test("a contact's glyph says nothing of its type, even one identified before", () => {
  const presented = (kind: string | null) => [
    { ...report(7, CONTACT_AIR, true), kind, opacity: 1, retiring: false },
  ];
  expect(contactLayer(presented(HELICOPTER), flat, facing)).toEqual(
    contactLayer(presented(null), flat, facing),
  );
});

test("an enemy the side has not found leaves no mark", () => {
  const m = indicators(observation(0, contacts), new ContactPresentation(TICK_HZ, 3));
  expect(marked(m, HIDDEN)).toBe(false);
});

test("a remembered contact keeps its glyph while its presentation holds it, then goes", () => {
  const presentation = new ContactPresentation(TICK_HZ, gameContactStyle.fade_s);
  indicators(observation(0, [...contacts, report(5, REMEMBERED)]), presentation);
  const fading = indicators(observation(TICK_HZ, contacts), presentation);
  expect(marked(fading, REMEMBERED, ["translucent"]), "fading, as the live view holds it").toBe(
    true,
  );
  const gone = TICK_HZ * (gameContactStyle.fade_s + 1);
  expect(marked(indicators(observation(gone, contacts), presentation), REMEMBERED)).toBe(false);
});
