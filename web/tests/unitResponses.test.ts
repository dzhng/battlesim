// @vitest-environment node
// The battle's side of the units' answers: what the player did and what the
// side observes, turned into the lines the battle's sound is asked for, and
// the voice sets it loads.
import { expect, test } from "vitest";
import { UnitResponses } from "@apps/battle-lab/src/unitResponses";
import type { ResponseRequest } from "@packages/battle-audio/src/voices/responses";
import type { ObservationView, OwnUnitView } from "@web/battle/sim/observationView";
import type { CommandAck, Order } from "@web/battle/sim/protocol";
import { UNITS } from "./catalog";

// test_tank (200) outvalues test_rifle (100), which outvalues test_supply (60).
const ASSIGNMENTS = {
  test_tank: "tanks",
  test_rifle: "rifles",
  test_supply: { us: "trucks_us", eastern: "trucks_ru" },
};

const unit = (id: number, kind: string, extra: Partial<OwnUnitView> = {}): OwnUnitView =>
  ({
    id,
    kind,
    goal: null,
    withdrawing: false,
    hp: kind === "test_rifle" ? 0 : UNITS.hull(kind)!.hp,
    memberIds: kind === "test_rifle" ? [1, 2, 3, 4, 5, 6, 7, 8] : [],
    suppression: "none",
    ...extra,
  }) as OwnUnitView;
const view = (tick: number, own: OwnUnitView[], pending: string[] = []) =>
  ({
    tick,
    own,
    skirmish: {
      phase: "preparation",
      players: [
        {
          index: 0,
          id: "blue-1",
          controller: "human",
          ready: false,
          credits: 1000,
          unitCount: 0,
          cap: 30,
          pending: pending.map((kind, i) => ({
            id: i,
            kind,
            owner: 0,
            destination: [0, 0],
            confirmedTick: 0,
            blocked: false,
            arrivalTick: 0,
          })),
        },
      ],
      preparationRemainingS: 60,
      scores: [0, 0],
      result: null,
      objectives: [],
      report: null,
    },
  }) as unknown as ObservationView;

function setup(faction: "us" | "eastern" | null = "eastern") {
  const asked: ResponseRequest[] = [];
  const loaded = new Set<string>();
  const responses = new UnitResponses(
    {
      respond: (request) => asked.push(request),
      loadVoices: (sets) => {
        for (const set of sets) loaded.add(set);
      },
    },
    UNITS,
    faction,
    10,
    ASSIGNMENTS,
  );
  const said = () => asked.splice(0).map((r) => `${r.speaker} ${r.trigger} ${r.intensity}`);
  return { responses, loaded, said };
}
const ack = (seq: number, reason?: string): CommandAck => ({
  seq,
  applied_tick: 0,
  error: reason ? { reason } : null,
  refused: [],
});

test("the sets loaded are those the side's units and its pending purchases speak with, for its faction", () => {
  const { responses, loaded } = setup("eastern");
  responses.observed(view(0, [unit(1, "test_rifle"), unit(2, "test_supply")], ["test_tank"]));
  expect([...loaded].sort()).toEqual(["rifles", "tanks", "trucks_ru"]);
});

test("a box selection is answered by its highest-value unit, which then answers the group's orders", () => {
  const { responses, said } = setup();
  responses.observed(
    view(0, [unit(1, "test_rifle"), unit(2, "test_tank"), unit(3, "test_supply")]),
  );
  responses.selected([1, 2, 3], null);
  expect(said()).toEqual(["own/2 selected steady"]);
  const move: Order = { kind: "move", units: [1, 2, 3], goal: [5, 5] } as Order;
  responses.acknowledged(move, ack(1), false);
  expect(said()).toEqual(["own/2 move steady"]);
  responses.acknowledged(move, ack(2, "no_valid_destination"), false);
  expect(said()).toEqual(["own/2 refused steady"]);
  // The clicked unit answers, and leads the next orders.
  responses.selected([1], 1);
  responses.acknowledged({ kind: "stop", units: [1, 2] } as Order, ack(3), false);
  expect(said()).toEqual(["own/1 selected steady", "own/1 stop steady"]);
});

test("a unit with no voice set stays silent", () => {
  const { responses, said } = setup();
  responses.observed(view(0, [unit(1, "test_jeep")]));
  responses.selected([1], 1);
  expect(said()).toEqual([]);
});

test("a hurt or suppressed unit answers stressed", () => {
  const { responses, said } = setup();
  responses.observed(
    view(0, [
      unit(1, "test_rifle", { suppression: "pinned" }),
      unit(2, "test_tank", { hp: UNITS.hull("test_tank")!.hp / 2 }),
      unit(3, "test_rifle", { memberIds: [1, 2, 3, 4, 5] }),
    ]),
  );
  for (const id of [1, 2, 3]) responses.selected([id], id);
  expect(said()).toEqual([
    "own/1 selected stressed",
    "own/2 selected stressed",
    "own/3 selected steady",
  ]);
});

test("a unit reports its arrival at the end of the move it was ordered", () => {
  const { responses, said } = setup();
  responses.observed(view(0, [unit(1, "test_tank")]));
  responses.acknowledged({ kind: "move", units: [1], goal: [90, 0] } as Order, ack(1), false);
  said();
  responses.observed(view(1, [unit(1, "test_tank", { goal: [90, 0] })]));
  responses.observed(view(200, [unit(1, "test_tank", { goal: [90, 0] })]));
  expect(said()).toEqual([]);
  responses.observed(view(201, [unit(1, "test_tank")]));
  expect(said()).toEqual(["own/1 arrived steady"]);
});
