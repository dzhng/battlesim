// Frontline lifecycle. Placement and deadlines belong to the pure model.
import { randomUUID } from "node:crypto";
import {
  place,
  seatsToFree,
  newGame,
  GONE_TIMEOUT_MS,
  IDLE_TIMEOUT_MS,
  type GameView,
} from "../../../packages/multiplayer/src/frontline.ts";
import type {
  BattleSetup,
  BattleTicket,
  Briefing,
} from "../../../packages/multiplayer/src/wire.ts";
import type { RefereeView } from "../../../web/src/battle/sim/protocol.ts";
import type { BattleHost } from "./battleHost/index.ts";
import type { BattleSlot, OnlineState } from "./online.ts";

type Reservation = {
  index: number;
  until: number;
  kind: "initial" | "join" | "resume";
  opening: boolean;
};
type Game = {
  setup: BattleSetup;
  host: BattleHost;
  slot: BattleSlot;
  view: GameView;
  referee: RefereeView;
  mapIdentity: string;
  tickHz: number;
  reservations: Map<string, Reservation>;
  drops: Set<string>;
  cancel: () => void;
};

/** `full`: no battle slot is free. `unavailable`: no battlefield could be
 *  made just now. `callsign`: the caller has not said hello. */
type JoinRefusal = "full" | "unavailable" | "callsign";

export function createFrontlineService(options: {
  /** How many maps a new game may draw before giving up. */
  mapAttempts: number;
  online: OnlineState;
  createBattle(setup: BattleSetup, id: string): BattleHost;
  random(): number;
}) {
  const { online } = options;
  const { clock, presence } = online;
  const games = new Map<string, Game>();
  const admitting = new Map<string, number>();
  const publishCounts = () =>
    online.publishFrontline({
      battles: games.size,
      commanders: [...games.values()].reduce(
        (count, g) =>
          count + g.view.seats.filter((s) => s.held && s.held.goneSince === null).length,
        0,
      ),
    });
  // Join and deploy include worker replies. Serialize admissions so two Play
  // requests cannot reserve the same seat while those replies are in flight.
  let pending: Promise<unknown> = Promise.resolve();
  let disposed = false;
  const serial = <T>(run: () => Promise<T>): Promise<T> => {
    const result = pending.then(run);
    pending = result.catch(() => {});
    return result;
  };
  const player = (g: Game, index: number) => g.setup.seats[index].player;
  const remove = (g: Game) => {
    if (!games.delete(g.view.battle)) return;
    publishCounts();
    g.cancel();
    g.slot.release();
    g.host.dispose();
    for (const who of new Set([...g.view.granted, ...g.reservations.keys()]))
      online.releaseIfForgotten(who);
  };
  const occupied = (g: Game) =>
    g.reservations.size || g.view.seats.some((s) => s.held && s.held.goneSince === null);
  const withdraw = (g: Game, index: number, why: "idle" | "leave" | "drop") => {
    const held = g.view.seats[index].held;
    if (!held || held.goneSince !== null) return;
    held.goneSince = clock.now();
    held.lastOrderAt = clock.now();
    void g.host.setController(player(g, index), "active_ai").catch(() => remove(g));
    if (why === "drop") g.drops.add(held.who);
    else void g.host.endSeat(player(g, index), why).catch(() => remove(g));
  };
  const seatOf = (g: Game, who: string) => {
    const index = g.view.seats.findIndex((s) => s.held?.who === who);
    if (
      index < 0 ||
      g.reservations.has(who) ||
      (g.view.seats[index].held!.goneSince !== null && !g.drops.has(who))
    )
      return;
    return { side: g.view.seats[index].side, player: player(g, index) };
  };
  const publishSeating = (g: Game) =>
    g.slot.seating(
      g.view.seats.flatMap((s) => {
        if (!s.held) return [];
        const seat = seatOf(g, s.held.who);
        return seat ? [[s.held.who, seat] as [string, typeof seat]] : [];
      }),
    );
  const maintain = (g: Game) => {
    if (!games.has(g.view.battle)) return;
    g.cancel();
    const now = clock.now();
    for (const [who, reservation] of g.reservations) {
      if (reservation.until > now) continue;
      g.reservations.delete(who);
      const seat = g.view.seats[reservation.index];
      seat.reservedUntil = null;
      if (reservation.kind === "initial") {
        seat.held = null;
        void g.host.setController(player(g, reservation.index), "active_ai").catch(() => remove(g));
      }
      online.releaseIfForgotten(who);
    }
    for (const free of seatsToFree(g.view, now)) {
      const index = g.view.seats.findIndex((s) => s.side === free.side && s.number === free.seat);
      const s = g.view.seats[index];
      if (g.reservations.has(s.held!.who)) continue;
      if (free.why === "idle") withdraw(g, index, "idle");
      else {
        const who = s.held!.who;
        s.held = null;
        g.drops.delete(who);
        online.releaseIfForgotten(who);
      }
    }
    if (!occupied(g)) return remove(g);
    publishCounts();
    void publishSeating(g).catch(() => remove(g));
    const deadlines = [...g.reservations.values()].map((r) => r.until);
    for (const s of g.view.seats)
      if (s.held && !g.reservations.has(s.held.who))
        deadlines.push(
          s.held.goneSince === null
            ? s.held.lastOrderAt + IDLE_TIMEOUT_MS
            : s.held.goneSince + GONE_TIMEOUT_MS,
        );
    g.cancel = clock.schedule(() => maintain(g), Math.max(0, Math.min(...deadlines) - now));
  };
  const reserve = (
    g: Game,
    who: string,
    index: number,
    kind: Reservation["kind"],
    opening: boolean,
  ) => {
    const until = clock.now() + GONE_TIMEOUT_MS;
    g.reservations.set(who, { index, kind, opening, until });
    g.view.seats[index].reservedUntil = until;
    presence.track(who);
    maintain(g);
  };
  const callsigns = (g: Game) =>
    Object.fromEntries(
      g.view.seats.flatMap((s, i) => {
        const who = s.held?.who ?? [...g.reservations].find(([, r]) => r.index === i)?.[0];
        return who ? [[player(g, i), online.callsign(who)!]] : [];
      }),
    );
  const ticket = (g: Game, index: number): BattleTicket => ({
    battle: g.view.battle,
    ...(online.battleAddress(g.host) && { address: online.battleAddress(g.host) }),
    setup: g.setup,
    mapIdentity: g.mapIdentity,
    side: g.view.seats[index].side,
    player: player(g, index),
    callsigns: callsigns(g),
  });
  const briefing = (g: Game, index: number, opening: boolean): Briefing => ({
    battle: g.view.battle,
    side: g.view.seats[index].side,
    seat: g.view.seats[index].number,
    credits:
      [...g.reservations.values()].find((r) => r.index === index)?.kind === "join"
        ? opening
          ? g.referee.opening_credits
          : 0
        : g.referee.seats[index].credits,
    scores: g.referee.scores,
    target: g.referee.target,
    minutesIn: g.referee.tick / (g.tickHz * 60),
    setup: g.setup,
    mapIdentity: g.mapIdentity,
    roster: g.view.seats.map((s, i) => {
      // A departed army is fighting under AI even while its human may return.
      const who =
        [...g.reservations].find(([, r]) => r.index === i)?.[0] ??
        (s.held?.goneSince === null ? s.held.who : undefined);
      return { side: s.side, callsign: who ? (online.callsign(who) ?? null) : null };
    }),
  });
  const unsubscribe = online.subscribe({
    changed(who) {
      if (presence.goneFor(who) !== null) return;
      for (const g of games.values()) {
        if (!g.drops.delete(who)) continue;
        const index = g.view.seats.findIndex((s) => s.held?.who === who);
        if (index < 0) continue;
        const held = g.view.seats[index].held!;
        held.goneSince = null;
        held.lastOrderAt = clock.now();
        void g.host.setController(player(g, index), "human").catch(() => remove(g));
        maintain(g);
      }
    },
    gone(who) {
      for (const g of games.values()) {
        if (g.reservations.has(who)) continue;
        const index = g.view.seats.findIndex((s) => s.held?.who === who);
        if (index >= 0) {
          withdraw(g, index, "drop");
          maintain(g);
        }
      }
    },
    remembers(who) {
      return (
        admitting.has(who) ||
        [...games.values()].some((g) => g.view.granted.includes(who) || g.reservations.has(who))
      );
    },
  });

  return {
    battle: online.battle,
    join(who: string): Promise<Briefing | { refused: JoinRefusal }> {
      admitting.set(who, (admitting.get(who) ?? 0) + 1);
      return serial<Briefing | { refused: JoinRefusal }>(async () => {
        if (!online.callsign(who)) return { refused: "callsign" };
        for (const g of games.values()) maintain(g);
        for (const g of games.values()) {
          const r = g.reservations.get(who);
          if (r) return briefing(g, r.index, r.opening);
        }
        await Promise.all(
          [...games.values()].map(async (g) => {
            try {
              g.referee = await g.host.referee();
              g.view.scores = g.referee.scores;
              g.view.target = g.referee.target;
            } catch {
              remove(g);
            }
          }),
        );
        const placement = place(
          [...games.values()].map((g) => g.view),
          who,
          clock.now(),
          options.random,
          online.slotFree(),
        );
        if ("full" in placement) return { refused: "full" };
        if ("create" in placement) {
          const slot = online.reserve()!;
          // The generator refuses some drawn maps, and another draw is
          // usually admitted: draw again, up to the game's own limit.
          for (let attempt = 0; attempt < options.mapAttempts; attempt++) {
            let host: BattleHost | undefined;
            try {
              const setup = newGame(options.random);
              // Let the placement owner choose the initial human, too.
              const view: GameView = {
                battle: randomUUID(),
                scores: [0, 0],
                target: Infinity,
                granted: [],
                seats: setup.seats.map((s) => ({
                  side: s.side,
                  number: setup.seats.filter((p) => p.side === s.side).indexOf(s) + 1,
                  held: null,
                  reservedUntil: null,
                })),
              };
              const initial = place([view], who, clock.now(), options.random, false);
              if (!("join" in initial)) throw new Error("a fresh battle has no seat");
              const index = view.seats.findIndex(
                (s) => s.side === initial.side && s.number === initial.seat,
              );
              setup.seats[index].controller = "human";
              host = options.createBattle(setup, view.battle);
              const ready = await host.ready;
              if (disposed) {
                host.dispose();
                slot.release();
                return { refused: "unavailable" };
              }
              const referee = await host.referee();
              const g: Game = {
                setup,
                host,
                slot,
                view,
                referee,
                mapIdentity: ready.mapIdentity,
                reservations: new Map(),
                drops: new Set(),
                cancel: () => {},
                tickHz: ready.tickHz,
              };
              view.target = referee.target;
              view.scores = referee.scores;
              view.granted.push(who);
              view.seats[index].held = { who, lastOrderAt: clock.now(), goneSince: null };
              games.set(view.battle, g);
              slot.serve({
                id: view.battle,
                host,
                seatOf: (browser) => seatOf(g, browser),
              });
              host.onEnd(() => remove(g));
              host.onOrder((id) => {
                const held = view.seats[setup.seats.findIndex((s) => s.player === id)]?.held;
                if (held && held.goneSince === null) {
                  held.lastOrderAt = clock.now();
                  maintain(g);
                }
              });
              reserve(g, who, index, "initial", true);
              return briefing(g, index, true);
            } catch {
              host?.dispose();
            }
          }
          slot.release();
          return { refused: "unavailable" };
        }
        const id = "resume" in placement ? placement.resume : placement.join;
        const g = games.get(id)!;
        const index = g.view.seats.findIndex(
          (s) => s.side === placement.side && s.number === placement.seat,
        );
        reserve(
          g,
          who,
          index,
          "resume" in placement ? "resume" : "join",
          "opening" in placement && placement.opening,
        );
        return briefing(g, index, "opening" in placement && placement.opening);
      }).finally(() => {
        const left = admitting.get(who)! - 1;
        if (left) admitting.set(who, left);
        else admitting.delete(who);
        online.releaseIfForgotten(who);
      });
    },
    deploy(who: string, id: string): Promise<BattleTicket | { refused: "expired" }> {
      return serial(async () => {
        const g = games.get(id);
        if (!g) return { refused: "expired" };
        maintain(g);
        if (!games.has(id)) return { refused: "expired" };
        const r = g.reservations.get(who);
        if (!r) {
          const i = g.view.seats.findIndex((s) => s.held?.who === who && s.held.goneSince === null);
          return i < 0 ? { refused: "expired" } : ticket(g, i);
        }
        try {
          if (r.kind === "join") await g.host.seatHuman(player(g, r.index), r.opening);
          else await g.host.setController(player(g, r.index), "human");
          if (!games.has(id)) return { refused: "expired" };
          g.reservations.delete(who);
          const s = g.view.seats[r.index];
          s.reservedUntil = null;
          s.held = { who, lastOrderAt: clock.now(), goneSince: null };
          g.drops.delete(who);
          if (r.opening && !g.view.granted.includes(who)) g.view.granted.push(who);
          presence.expectStream(who);
          maintain(g);
          await publishSeating(g);
          return ticket(g, r.index);
        } catch {
          remove(g);
          return { refused: "expired" };
        }
      });
    },
    leave(who: string, id: string) {
      const g = games.get(id);
      if (!g) return { ok: true } as const;
      const r = g.reservations.get(who);
      if (r) {
        g.reservations.delete(who);
        g.view.seats[r.index].reservedUntil = null;
        if (r.kind === "initial") {
          g.view.seats[r.index].held = null;
          void g.host.setController(player(g, r.index), "active_ai").catch(() => remove(g));
        }
      }
      const index = g.view.seats.findIndex((s) => s.held?.who === who);
      if (index >= 0) withdraw(g, index, "leave");
      maintain(g);
      return { ok: true } as const;
    },
    dispose() {
      disposed = true;
      for (const g of games.values()) remove(g);
      unsubscribe();
    },
  };
}
export type FrontlineService = ReturnType<typeof createFrontlineService>;
