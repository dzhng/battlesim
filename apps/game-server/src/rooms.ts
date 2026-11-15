import { battlePlayers } from "../../../packages/skirmish-setup/src/seats.ts";
import { randomUUID } from "node:crypto";
import type { BattleSetup } from "../../../packages/multiplayer/src/wire.ts";
import type { BattleHost } from "./battleHost/index.ts";
import type { OnlineState, BattleSlot } from "./online.ts";
import {
  createRoom,
  battleSetup,
  recordBattle,
  roomCode,
  roomReduce,
  roomView,
  type Room,
  type RoomRefusal,
} from "../../../packages/multiplayer/src/room.ts";
import {
  KEYFRAME,
  jsonRecord,
  recordSource,
  type RecordSource,
} from "../../../packages/multiplayer/src/stream/index.ts";
import type { RoomAction, RoomList } from "../../../packages/multiplayer/src/wire.ts";
import { seedFrom } from "../../../web/src/maps/source.ts";
import { PRESENCE_GRACE_MS } from "./presence.ts";

/** Bound wrong guesses before a client can search a private code's small space. */
export const THROTTLE_MAX_GUESSES = 5;
/** Let a mistyping client try again after a short pause. */
export const THROTTLE_WINDOW_MS = 10_000;
/** Reclaim a battle slot after a minute without any connected human member. */
export const EMPTY_ROOM_TIMEOUT_MS = 60_000;

type Deliver = Parameters<RecordSource["attach"]>[1];
type StoredRoom = {
  room: Room;
  slot: BattleSlot;
  source: RecordSource;
  idle: { cancel: () => void } | null;
  battle: { id: string; host: BattleHost; controllers: Map<string, string> } | null;
};

export function createRoomService(options: {
  createBattle: (setup: BattleSetup) => BattleHost;
  online: OnlineState;
  random: () => number;
}) {
  const { online } = options;
  const { clock, presence, releaseIfForgotten } = online;
  const rooms = new Map<string, StoredRoom>();
  const guesses = new Map<string, { count: number; cancel: () => void }>();

  const roomList = (): RoomList => ({
    frontline: online.frontline(),
    rooms: [...rooms.values()]
      .map(({ room }) => room)
      .filter((room) => room.visibility === "public" && room.phase === "lobby")
      .map((room) => ({
        code: room.code,
        host: room.members.find((member) => member.who === room.host)?.callsign ?? "",
        map: room.lobby.map,
        blue: room.lobby.seats
          .filter((seat) => seat.side === "blue")
          .map((seat) => seat.occupant.kind),
        red: room.lobby.seats
          .filter((seat) => seat.side === "red")
          .map((seat) => seat.occupant.kind),
      })),
  });

  const append = (source: RecordSource, value: unknown) =>
    source.append("state", KEYFRAME, jsonRecord("state", value).payload);
  // A single source numbers each room/list and retains only its current snapshot.
  const listSource = recordSource(
    null,
    () => ({}),
    () => publishList(),
  );
  const publishList = () => append(listSource, roomList());
  const unlistenList = online.onFrontlineChanged(publishList);

  const remove = (code: string, reason = "room_deleted") => {
    const stored = rooms.get(code);
    if (!stored) return;
    stored.idle?.cancel();
    rooms.delete(code);
    stored.slot.release();
    stored.battle?.host.dispose();
    stored.source.end(reason);
    for (const member of stored.room.members) releaseIfForgotten(member.who);
    publishList();
  };
  const updateIdle = (stored: StoredRoom) => {
    const absences = stored.room.members.map((member) => presence.goneFor(member.who));
    if (absences.includes(null)) {
      stored.idle?.cancel();
      stored.idle = null;
      return;
    }
    if (stored.idle) return;
    const absentFor = absences.length ? Math.min(...(absences as number[])) : 0;
    stored.idle = {
      cancel: clock.schedule(
        () => remove(stored.room.code),
        Math.max(0, EMPTY_ROOM_TIMEOUT_MS - absentFor),
      ),
    };
  };
  const publishRoom = (stored: StoredRoom) => {
    // The source carries only a public view; each reader projects its own `you` and ticket.
    append(stored.source, roomView(stored.room, ""));
    publishList();
    updateIdle(stored);
  };
  const controller = (stored: StoredRoom, who: string, kind: "human" | "active_ai") => {
    const battle = stored.battle;
    if (!battle) return;
    // The model owns the seat-to-player projection, including dropped open seats.
    const member = stored.room.members.find((m) => m.who === who);
    if (!member) return;
    const player = battlePlayers(stored.room.lobby.seats).playerOf[member.seat]!;
    if (battle.controllers.get(player) === kind) return;
    battle.controllers.set(player, kind);
    battle.host.setController(player, kind);
  };
  const startBattle = (stored: StoredRoom) => {
    try {
      const setup = battleSetup(stored.room);
      const host = options.createBattle(setup);
      const id = randomUUID();
      stored.battle = {
        id,
        host,
        controllers: new Map(setup.seats.map((s) => [s.player, s.controller])),
      };
      const ended = (reason: string) => {
        if (rooms.get(stored.room.code) === stored) remove(stored.room.code, reason);
      };
      host.onEnd((end) => ended(end.kind === "finished" ? "battle_finished" : "battle_failed"));
      for (const member of stored.room.members) {
        const absent = presence.goneFor(member.who);
        if (absent !== null && absent >= PRESENCE_GRACE_MS)
          controller(stored, member.who, "active_ai");
      }
      void host.ready
        .then(({ mapIdentity }) => {
          if (rooms.get(stored.room.code) !== stored) return;
          stored.room = recordBattle(stored.room, { id, mapIdentity });
          stored.slot.serve({
            id,
            host,
            seatOf(who) {
              const view = roomView(stored.room, who);
              if (view.phase.kind !== "battle") return;
              const { side, player } = view.phase.ticket;
              return { side, player };
            },
          });
          publishRoom(stored);
        })
        .catch(() => ended("battle_failed"));
    } catch {
      remove(stored.room.code, "battle_failed");
    }
  };
  const transition = (
    stored: StoredRoom,
    who: string,
    action: Parameters<typeof roomReduce>[2],
  ) => {
    const next = roomReduce(stored.room, who, action, clock.now());
    if ("refused" in next) return next;
    if (next !== stored.room) {
      const previous = stored.room;
      stored.room = next;
      publishRoom(stored);
      for (const member of previous.members) releaseIfForgotten(member.who);
      if (previous.phase === "lobby" && next.phase === "starting") startBattle(stored);
    }
    return { ok: true } as const;
  };
  const unsubscribe = online.subscribe({
    changed(who) {
      for (const stored of rooms.values()) {
        if (stored.room.members.some((member) => member.who === who)) {
          updateIdle(stored);
          if (presence.goneFor(who) === null) controller(stored, who, "human");
        }
      }
    },
    gone(who) {
      for (const stored of rooms.values()) {
        if (stored.room.phase === "lobby") transition(stored, who, { kind: "leave" });
        else controller(stored, who, "active_ai");
      }
    },
    remembers: (who) =>
      [...rooms.values()].some(({ room }) => room.members.some((m) => m.who === who)),
  });

  return {
    presence,
    battle: online.battle,
    create(who: string, visibility: Room["visibility"]): { code: string } | RoomRefusal {
      if (!online.slotFree()) return { refused: "full" };
      const callsign = online.callsign(who);
      if (!callsign) return { refused: "callsign" };
      presence.track(who);
      let code: string;
      do {
        code = roomCode(options.random);
      } while (rooms.has(code));
      const source = recordSource(
        null,
        () => ({}),
        () => append(source, roomView(stored.room, "")),
      );
      const stored: StoredRoom = {
        slot: online.reserve()!,
        room: createRoom(
          { who, callsign },
          visibility,
          code,
          seedFrom(options.random),
          clock.now(),
        ),
        source,
        idle: null,
        battle: null,
      };
      rooms.set(code, stored);
      updateIdle(stored);
      publishList();
      return { code };
    },
    listStream(who: string, after: number | null, deliver: Deliver) {
      const closePresence = presence.open(who);
      const detach = listSource.attach(after, deliver);
      return () => {
        detach();
        closePresence();
      };
    },
    roomStream(
      who: string,
      code: string,
      address: string,
      after: number | null,
      deliver: Deliver,
    ): { close: () => void } | RoomRefusal {
      const stored = rooms.get(code);
      const returning = stored?.room.members.some((member) => member.who === who);
      // A resume keeps an existing membership; it must never silently join again.
      if (after !== null && !returning) return { refused: "removed" };
      if (!returning && (guesses.get(address)?.count ?? 0) >= THROTTLE_MAX_GUESSES)
        return { refused: "throttled" };
      if (!stored) {
        let guess = guesses.get(address);
        if (!guess) {
          guess = {
            count: 0,
            cancel: clock.schedule(() => guesses.delete(address), THROTTLE_WINDOW_MS),
          };
          guesses.set(address, guess);
        }
        guess.count++;
        return { refused: "unknown" };
      }
      const callsign = online.callsign(who);
      if (!callsign) return { refused: "callsign" };
      const joined = transition(stored, who, { kind: "join", callsign });
      if ("refused" in joined) {
        if (joined.refused === "this room has started") return { refused: "started" };
        if (joined.refused === "this room is full") return { refused: "no_open_seat" };
        return joined;
      }
      const closePresence = presence.open(who);
      const detach = stored.source.attach(after, (record) =>
        deliver(
          record.kind === "state"
            ? { ...record, payload: jsonRecord("state", roomView(stored.room, who)).payload }
            : record,
        ),
      );
      return {
        close: () => {
          detach();
          closePresence();
        },
      };
    },
    act(who: string, code: string, action: RoomAction): { ok: true } | RoomRefusal {
      const stored = rooms.get(code);
      return stored ? transition(stored, who, action) : { refused: "unknown" };
    },
    dispose() {
      for (const stored of rooms.values()) {
        stored.idle?.cancel();
        stored.battle?.host.dispose();
        stored.slot.release();
        stored.source.end("disposed");
      }
      for (const guess of guesses.values()) guess.cancel();
      rooms.clear();
      guesses.clear();
      listSource.end("disposed");
      unsubscribe();
      unlistenList();
    },
  };
}
export type RoomService = ReturnType<typeof createRoomService>;
