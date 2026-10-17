// What a unit says back to its owner (R1–R12): the owner's events, mapped to
// the line a unit asks `Chatter` for. Only the owner hears these, wherever the
// camera is, so a response has no position. Which line plays, and whether, is
// still the chatter's call; this only decides what is asked and by whom:
//
// - one unit answers for a group: the lead (the unit the player clicked, or
//   the one that answered the selection) when the order includes it, else the
//   group's highest-value unit, ties to the lower id;
// - an order is answered once it is accepted (R2–R10), or refused when the
//   refusal is the player's doing (R12);
// - an arrival (R11) is a move's end after a real journey, reported once per
//   journey by the first of its units to get there (`Arrivals`).
import type { VoiceRequest } from "./chatter";
import type { RESPONSE_TRIGGERS } from "./voiceCatalog";

type ResponseTrigger = (typeof RESPONSE_TRIGGERS)[number];

/** An own unit that can answer. */
export interface ResponseUnit {
  id: number;
  /** The voice set it speaks with (`voices.json` assignments). */
  set: string;
  /** What it cost: in a group, the highest-value unit answers. */
  value: number;
  /** Health at half or less, or suppressed: it answers in its stressed voice. */
  stressed: boolean;
}

/** An order as the battle sends it; a fire-policy order names its policy. */
export interface OrderLike {
  kind: string;
  policy?: string;
  units?: readonly number[];
}

export type ResponseEvent =
  /** The selection gained `units`. */
  | { kind: "selected"; units: readonly ResponseUnit[]; lead?: number | null }
  /** The authority accepted order `id` for `units`. */
  | {
      kind: "accepted";
      order: OrderLike;
      units: readonly ResponseUnit[];
      lead?: number | null;
      id: number;
    }
  /** The authority refused order `id`, for `reason` (`OrderError.reason`). */
  | {
      kind: "refused";
      reason: string;
      units: readonly ResponseUnit[];
      lead?: number | null;
      id: number;
    }
  /** `unit` reached the end of journey `group` (`Arrivals`). */
  | { kind: "arrived"; unit: ResponseUnit; group: string };

/** A request as the battle makes it; the sink stamps its moment. */
export type ResponseRequest = Omit<VoiceRequest, "at">;

const ORDER_TRIGGERS: Record<string, ResponseTrigger> = {
  move: "move",
  attack: "attack",
  attack_move: "attack_move",
  stop: "stop",
  garrison: "enter_building",
  occupy_building: "enter_building",
  exit_building: "leave_building",
  board: "board",
  collect: "board",
  unload: "unload",
  refund: "withdraw",
};
const POLICY_TRIGGERS: Record<string, ResponseTrigger> = {
  fire_at_will: "weapons_free",
  return_fire_only: "hold_fire",
};

/** Refusals the player caused with the order itself (`OrderError` in
 *  `crates/contract/src/command.rs`), so a unit says it can't. The rest are
 *  the match's (credits, limits, a finished match), the input path's (rate,
 *  sequence, a replay) or stale ids, which no unit would answer. */
export const REFUSALS: ReadonlySet<string> = new Set([
  "out_of_bounds",
  "no_valid_destination",
  "unknown_target",
  "air_contact",
  "not_infantry",
  "not_a_building",
  "capacity_full",
  "one_squad_per_building",
  "building_occupied",
  "no_room_to_unload",
]);

/** The line an order asks for, or null when no unit answers it. */
function orderTrigger({ kind, policy }: OrderLike): ResponseTrigger | null {
  if (kind === "set_engagement") return (policy && POLICY_TRIGGERS[policy]) || null;
  return ORDER_TRIGGERS[kind] ?? null;
}

/** Who answers for `units`: the lead if among them, else the highest value, then the lower id. */
export function answerer(
  units: readonly ResponseUnit[],
  lead?: number | null,
): ResponseUnit | null {
  const named = units.find((unit) => unit.id === lead);
  if (named) return named;
  let best: ResponseUnit | null = null;
  for (const unit of units)
    if (!best || unit.value > best.value || (unit.value === best.value && unit.id < best.id))
      best = unit;
  return best;
}

const request = (
  unit: ResponseUnit,
  trigger: ResponseTrigger,
  groupKey: string,
): ResponseRequest => ({
  tier: "response",
  speaker: `own/${unit.id}`,
  set: unit.set,
  trigger,
  intensity: unit.stressed ? "stressed" : "steady",
  groupKey,
});

/** The line `event` asks for, or null when nothing answers it. */
export function responseFor(event: ResponseEvent): ResponseRequest | null {
  if (event.kind === "arrived") return request(event.unit, "arrived", `arrived/${event.group}`);
  const trigger =
    event.kind === "selected"
      ? "selected"
      : event.kind === "accepted"
        ? orderTrigger(event.order)
        : REFUSALS.has(event.reason)
          ? event.reason === "no_room_to_unload"
            ? "no_room_to_unload"
            : "refused"
          : null;
  const unit = answerer(event.units, event.lead);
  if (!trigger || !unit) return null;
  // Selections in quick succession are one answer, as an order's units are.
  return request(unit, trigger, event.kind === "selected" ? "selected" : `order/${event.id}`);
}

/** A move's end shorter than this, in seconds under way, is a nudge, not an arrival. */
export const ARRIVED_AFTER_S = 3;
/** Orders whose end is an arrival. */
const JOURNEYS = new Set([
  "move",
  "attack_move",
  "garrison",
  "occupy_building",
  "board",
  "collect",
  "unload",
]);
/** Orders that end or replace a journey without arriving. */
const HALTS = new Set(["stop", "attack", "refund", "exit_building"]);

interface Journey {
  group: string;
  /** When the unit was first seen under way; null until then. */
  since: number | null;
}

/**
 * R11 from what the side observes: a unit arrives when its ordered move ends
 * (`OwnUnit.goal` goes from a goal to none) after at least `afterS` under way.
 * A purchased unit entering the field under way is on a journey of its own. A
 * stop, an attack (which may chase) or a withdrawal ends a journey silently,
 * and a unit that starts moving with no order of the player's reports nothing.
 */
export class Arrivals {
  private readonly known = new Set<number>();
  private readonly journeys = new Map<number, Journey>();
  private readonly reported = new Set<string>();

  constructor(private readonly afterS = ARRIVED_AFTER_S) {}

  /** An accepted order, `group` naming its journey. A queued order leaves the current one running. */
  ordered(order: OrderLike, queued: boolean, group: string) {
    for (const id of order.units ?? []) {
      if (JOURNEYS.has(order.kind)) {
        if (!queued || !this.journeys.has(id)) this.journeys.set(id, { group, since: null });
      } else if (HALTS.has(order.kind) && !queued) this.journeys.delete(id);
    }
  }

  /** The side's units at `time` seconds, each under way or not: who arrived, one per journey. */
  observe(
    units: readonly { id: number; moving: boolean }[],
    time: number,
  ): { id: number; group: string }[] {
    const arrived: { id: number; group: string }[] = [];
    const present = new Set<number>();
    for (const { id, moving } of units) {
      present.add(id);
      if (!this.known.has(id)) {
        this.known.add(id);
        if (moving && !this.journeys.has(id))
          this.journeys.set(id, { group: `unit/${id}`, since: null });
      }
      const journey = this.journeys.get(id);
      if (!journey) continue;
      if (moving) {
        journey.since ??= time;
        continue;
      }
      if (journey.since === null) continue;
      this.journeys.delete(id);
      if (time - journey.since >= this.afterS && !this.reported.has(journey.group)) {
        this.reported.add(journey.group);
        arrived.push({ id, group: journey.group });
      }
    }
    for (const id of this.journeys.keys()) if (!present.has(id)) this.journeys.delete(id);
    return arrived;
  }
}
