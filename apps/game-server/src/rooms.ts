import { cpus, totalmem } from "node:os";
import {
  createRoom,
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
import { createPresence, PRESENCE_GRACE_MS, type PresenceClock } from "./presence.ts";

/** Slice 01's memory budget leaves room for a battle to grow toward an hour. */
export const BYTES_PER_BATTLE = 2 * 1024 ** 3;
/** Bound wrong guesses before a client can search a private code's small space. */
export const THROTTLE_MAX_GUESSES = 5;
/** Let a mistyping client try again after a short pause. */
export const THROTTLE_WINDOW_MS = 10_000;
/** Reclaim a battle slot after a minute without any connected human member. */
export const EMPTY_ROOM_TIMEOUT_MS = 60_000;

export function battleCapacity(cores = cpus().length, memoryBytes = totalmem()): number {
  return Math.max(0, Math.min(cores - 1, Math.floor(memoryBytes / BYTES_PER_BATTLE)));
}

type Deliver = Parameters<RecordSource["attach"]>[1];
type StoredRoom = {
  room: Room;
  source: RecordSource;
  idle: { cancel: () => void } | null;
};

export function createRoomService(options: {
  capacity: number;
  clock: PresenceClock;
  callsign: (who: string) => string | undefined;
  releaseCallsign: (who: string) => void;
  random: () => number;
}) {
  const { clock } = options;
  const rooms = new Map<string, StoredRoom>();
  const guesses = new Map<string, { count: number; cancel: () => void }>();

  const roomList = (): RoomList => ({
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

  const releaseIfForgotten = (who: string) => {
    const absent = presence.goneFor(who);
    if (
      absent === null ||
      absent < PRESENCE_GRACE_MS ||
      [...rooms.values()].some(({ room }) => room.members.some((member) => member.who === who))
    )
      return;
    options.releaseCallsign(who);
    presence.forget(who);
  };
  const remove = (code: string) => {
    const stored = rooms.get(code)!;
    stored.idle?.cancel();
    rooms.delete(code);
    stored.source.end("room_deleted");
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
    }
    return { ok: true } as const;
  };
  const presence = createPresence(
    clock,
    (who) => {
      for (const stored of rooms.values()) {
        if (stored.room.members.some((member) => member.who === who)) updateIdle(stored);
      }
    },
    (who) => {
      for (const stored of rooms.values()) transition(stored, who, { kind: "leave" });
      releaseIfForgotten(who);
    },
  );

  return {
    presence,
    create(who: string, visibility: Room["visibility"]): { code: string } | RoomRefusal {
      if (rooms.size >= options.capacity) return { refused: "full" };
      const callsign = options.callsign(who);
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
        room: createRoom(
          { who, callsign },
          visibility,
          code,
          seedFrom(options.random),
          clock.now(),
        ),
        source,
        idle: null,
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
      const callsign = options.callsign(who);
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
        stored.source.end("disposed");
      }
      for (const guess of guesses.values()) guess.cancel();
      rooms.clear();
      guesses.clear();
      listSource.end("disposed");
      presence.dispose();
    },
  };
}
export type RoomService = ReturnType<typeof createRoomService>;
