// The match report's specimens for the UI gallery: a finished battle of any
// pairing, built from a link's controller codes, its timeline a plausible
// battle's (each player buying in waves, kills crossing the front, a few
// refunds) and every player's totals summed from it, as the referee keeps
// them. Test units only: the roster grows.
import { shippedRules } from "@web/shippedRules";
import type { Seat } from "@web/battle/present/playerName";
import type { MatchEventView, SkirmishView } from "@web/battle/sim/observationView";

const KINDS = [
  ["test_tank", 300],
  ["test_rifle", 110],
  ["test_jeep", 90],
  ["test_at", 160],
  ["test_recon", 120],
  ["test_supply", 80],
] as const;

export interface ReportSpecimen {
  match: SkirmishView;
  seats: Seat[];
  /** The human the client plays as; null when it watches AI play. */
  viewer: string | null;
}

/** A finished battle between `blue` and `red` (controller codes: `h` a
 *  human, `a` an AI), won by `result`. */
export function reportSpecimen(
  blue: string,
  red: string,
  result: SkirmishView["result"],
): ReportSpecimen {
  const seats = (["blue", "red"] as const).flatMap((side) =>
    [...(side === "blue" ? blue : red)].map((code, k) => ({
      id: `${side}-${k + 1}`,
      side,
      controller: code === "h" ? ("human" as const) : ("active_ai" as const),
    })),
  );
  const minute = 60 * shippedRules.tick_hz;
  const events: MatchEventView[] = [];
  const alive: { side: "blue" | "red"; owner: number; unitId: number; kind: number }[] = [];
  let unitId = 0;
  // Three waves of purchases a few minutes apart, each player's units
  // arriving a few seconds after the last; kills in between.
  for (let wave = 0; wave < 3; wave++) {
    const at = wave * 4 * minute + 30;
    seats.forEach((seat, owner) => {
      for (let k = 0; k < 2 + ((owner + wave) % 3); k++) {
        const kind = (owner * 7 + wave * 3 + k * 5) % KINDS.length;
        const tick = at + (k * seats.length + owner) * 40;
        events.push({
          kind: "spawn",
          tick,
          side: seat.side,
          unitId,
          unitKind: KINDS[kind][0],
          owner,
          amount: KINDS[kind][1],
        });
        alive.push({ side: seat.side, owner, unitId: unitId++, kind });
      }
    });
    for (let k = 0; k < seats.length + wave; k++) {
      const tick = at + minute + k * 13 * shippedRules.tick_hz;
      // Only what fights kills: tanks, rifles, anti-tank teams.
      const fighters = alive.filter(
        (u) => KINDS[u.kind][0] !== "test_supply" && KINDS[u.kind][0] !== "test_jeep",
      );
      const killer = fighters[(k * 5 + wave) % fighters.length];
      const victims = alive.filter((u) => u.side !== killer.side);
      const victim = victims[(k * 3 + wave) % victims.length];
      if (!victim) continue;
      alive.splice(alive.indexOf(victim), 1);
      events.push({
        kind: "kill",
        tick,
        side: victim.side,
        unitId: victim.unitId,
        unitKind: KINDS[victim.kind][0],
        owner: victim.owner,
        victimCost: KINDS[victim.kind][1],
        killer: {
          side: killer.side,
          unitId: killer.unitId,
          unitKind: KINDS[killer.kind][0],
          owner: killer.owner,
          cost: KINDS[killer.kind][1],
        },
      });
    }
    const back = alive[(wave * 7) % alive.length];
    alive.splice(alive.indexOf(back), 1);
    events.push({
      kind: "refund",
      tick: at + 3 * minute,
      side: back.side,
      unitId: back.unitId,
      unitKind: KINDS[back.kind][0],
      owner: back.owner,
      amount: KINDS[back.kind][1] * 0.56,
    });
  }
  events.sort((a, b) => a.tick - b.tick);
  const totals = seats.map(() => ({ spawned: 0, spent: 0, killed: 0, refunded: 0 }));
  for (const e of events) {
    if (e.kind === "spawn") {
      totals[e.owner!].spawned++;
      totals[e.owner!].spent += e.amount;
    } else if (e.kind === "refund") totals[e.owner!].refunded += e.amount;
    else if (e.killer) totals[e.killer.owner!].killed++;
  }
  const human = seats.find((seat) => seat.controller === "human");
  return {
    seats,
    viewer: human?.id ?? null,
    match: {
      phase: "finished",
      players: [],
      preparationRemainingS: 0,
      scores: result === "red" ? [612, 1000] : result === "blue" ? [1000, 487] : [1000, 1000],
      result,
      objectives: [],
      report: { totals, events },
    },
  };
}
