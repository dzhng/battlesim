// What a unit says back to its owner (R1–R12): the owner's events, mapped to
// the line a unit asks `Chatter` for. Only the owner hears these, wherever the
// camera is, so a response has no position. Which line plays, and whether, is
// still the chatter's call; this only decides what is asked and by whom:
//
// - one unit answers for a group: the lead (the unit the player clicked, or
//   the one that answered the selection) when the order includes it, else the
//   group's highest-value unit, ties to the lower id;
// - an order is answered once it is accepted (R2–R10), or refused when the
//   refusal is the player's doing (R12); a unit an order refuses on its own
//   answers with its refusal, apart from the order's answer;
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

/** What an order acts on, as the battle names it: an attack's target. */
interface Aimed {
  kind: string;
  id?: number;
}

/** An order as the battle sends it; a fire-policy order names its policy,
 *  an attack its target. */
export interface OrderLike {
  kind: string;
  policy?: string;
  units?: readonly number[];
  target?: Aimed;
}

/** A unit's current order as the side publishes it (`OwnUnit.steps[0]`):
 *  its kind and what it acts on, never where. */
export interface FrontStep {
  kind: string;
  target?: Aimed;
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
  /** The authority refused `units` for `reason` (`OrderError.reason`): a
   *  whole order, the units an order left out, or a refusal on arrival.
   *  `group` names the moment (`order/<seq>`, `tick/<tick>`); a moment's
   *  refusals for one reason are one answer. */
  | {
      kind: "refused";
      reason: string;
      units: readonly ResponseUnit[];
      lead?: number | null;
      group: string;
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
  "no_room_to_unload",
  "too_close",
  "not_a_carrier",
  "aboard",
  "wrong_player",
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
  const group =
    event.kind === "selected"
      ? "selected"
      : event.kind === "accepted"
        ? `order/${event.id}`
        : `refused/${event.group}/${event.reason}`;
  return request(unit, trigger, group);
}

/** A move's end shorter than this, in seconds under way, is a nudge, not an arrival. */
export const ARRIVED_AFTER_S = 3;
/** Orders whose end is an arrival. */
const JOURNEYS = new Set(["move", "attack_move", "occupy_building", "board", "collect", "unload"]);
/** Orders that end without arriving, by the step each publishes. */
const ERRANDS: Record<string, string> = {
  attack: "attack",
  exit_building: "exit",
  refund: "return_to_base",
};

/** A step's kind and the id of what it acts on, if the side names one. */
interface StepIdentity {
  kind: string;
  id: string | null;
}

/** A contact names no lasting target: identified mid-attack, the sim turns
 *  the attack onto the unit, so any attack at the front is still it. */
const stepIdentity = (kind: string, target?: Aimed): StepIdentity => ({
  kind,
  id: target?.id === undefined || target.kind === "contact" ? null : `${target.kind}/${target.id}`,
});

/** The front step is still `errand`: the same kind, on the same target
 *  while the side names one (a lost target's attack publishes none). */
function still(errand: StepIdentity, front: FrontStep | null | undefined): boolean {
  if (!front || front.kind !== errand.kind) return false;
  const id = stepIdentity(front.kind, front.target).id;
  return errand.id === null || id === null || id === errand.id;
}

/** One of a unit's orders, in turn: a journey, which ends when the unit
 *  stops after being under way, or an errand (an attack, an exit, a return
 *  to base), which ends when the published front step stops being it and
 *  never arrives. */
interface Entry {
  /** A journey's group; null for an errand. */
  group: string | null;
  errand: StepIdentity | null;
  /** When the order applies, in seconds: until then the old order still
   *  shows, and an errand can't have ended. */
  from: number;
  /** When the unit was first seen under way on it; null until then. */
  since: number | null;
}

/**
 * R11 from what the side observes: a unit arrives when its ordered move ends
 * (`OwnUnit.goal` goes from a goal to none) after at least `afterS` under way.
 * A purchased unit entering the field under way is on a journey of its own.
 * Every accepted order takes its turn in the unit's queue: a queued order
 * starts when the one before it ends. A journey ends at the unit's stop,
 * since some journeys (an unload, a board, a collect) publish no goal to
 * watch; an attack (which may chase), an exit or a return to base ends when
 * the published front step leaves it, silently. A stop drops them all, and a
 * unit that starts moving with no order of the player's reports nothing.
 */
export class Arrivals {
  private readonly known = new Set<number>();
  /** Each unit's orders, the current one first. */
  private readonly orders = new Map<number, Entry[]>();
  private readonly reported = new Set<string>();

  constructor(private readonly afterS = ARRIVED_AFTER_S) {}

  /** An accepted order, `group` naming its journey, applying at `from`
   *  seconds. A queued order waits behind the current one. */
  ordered(order: OrderLike, queued: boolean, group: string, from = -Infinity) {
    const errand = ERRANDS[order.kind];
    const entry: Entry | null = JOURNEYS.has(order.kind)
      ? { group, errand: null, from, since: null }
      : errand
        ? { group: null, errand: stepIdentity(errand, order.target), from, since: null }
        : null;
    for (const id of order.units ?? []) {
      const current = this.orders.get(id);
      if (entry && queued && current) current.push({ ...entry });
      else if (entry) this.orders.set(id, [{ ...entry }]);
      else if (order.kind === "stop" && !queued) this.orders.delete(id);
    }
  }

  /** The side's units at `time` seconds, each under way or not, with its
   *  published front step: who arrived, one per journey. */
  observe(
    units: readonly { id: number; moving: boolean; front?: FrontStep | null }[],
    time: number,
  ): { id: number; group: string }[] {
    const arrived: { id: number; group: string }[] = [];
    const present = new Set<number>();
    for (const { id, moving, front } of units) {
      present.add(id);
      if (!this.known.has(id)) {
        this.known.add(id);
        if (moving && !this.orders.has(id))
          this.orders.set(id, [
            { group: `unit/${id}`, errand: null, from: -Infinity, since: null },
          ]);
      }
      const orders = this.orders.get(id) ?? [];
      // Each order that has ended gives way to the next, which may have
      // ended too: an errand done before it was seen at the front.
      for (let entry = orders[0]; entry; entry = orders[0]) {
        if (entry.errand) {
          if (time < entry.from || still(entry.errand, front)) break;
        } else if (moving) {
          entry.since ??= time;
          break;
        } else if (entry.since === null) break;
        orders.shift();
        const { group, since } = entry;
        if (group && since !== null && time - since >= this.afterS && !this.reported.has(group)) {
          this.reported.add(group);
          arrived.push({ id, group });
        }
      }
      if (!orders.length) this.orders.delete(id);
    }
    for (const id of this.orders.keys()) if (!present.has(id)) this.orders.delete(id);
    return arrived;
  }
}
