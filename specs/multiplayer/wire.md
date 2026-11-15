# The wire contract

Everything a page and the game server say to each other. The types live in
`packages/multiplayer/src/wire.ts` (new); this file is their rationale and
their shape at planning time. Where the two disagree after a slice lands, the
code is right and this file is updated in that slice.

Types named here that already exist are used as they are: `Order`,
`CommandAck`, `PurchasePlacement`, `MoveDestination`, `BuildingPlacement`,
`MovePreviewRequest`, `BuildingPreviewRequest`
(`web/src/battle/sim/protocol.ts`), `ControllerKind`
(`web/src/battle/prepare/protocol.ts`), `MapChoice` (`web/src/maps/source.ts`)
and `Faction` (`packages/scene-assets/src/units.ts`).

## Every request and response

- **Request header `X-Player`:** the browser id, 128 random bits in hex. It is
  the only credential. The server derives the seat from it and ignores any
  side, player or seat a body claims.
- **Response headers:** `Cache-Control: no-store, no-transform`;
  `Access-Control-Allow-Origin: *`; `Access-Control-Allow-Headers: X-Player,
  Content-Type`; `Access-Control-Max-Age: 7200`. Streams add
  `X-Accel-Buffering: no` and `Content-Type: application/octet-stream`.
- Bodies are JSON, except stream responses.
- A refusal is a normal response: `{ "refused": "<reason>" }`.
- Asking twice for a state you already have succeeds. A member reopening
  their room's stream is still in their seat, including after the battle has
  started; a second `join` returns the same reservation; a second `deploy`
  returns the same ticket; `leave` from a room you are not in is fine. A
  refusal is for a request that cannot be honoured, never for a repeat.

## Endpoints

| Method and path | Body | Answer |
|---|---|---|
| `POST /hello` | `{ build: BuildIdentity, callsign: string }` | `{ callsign }` (with `#2` on a clash), or refused `build` |
| `GET /rooms` | | state stream of `RoomList` |
| `POST /rooms` | `{ visibility: "public" \| "private" }` | `{ code: RoomCode }`, or refused `full` |
| `GET /rooms/:code` | | state stream of `RoomView`; opening it joins the room. A newcomer is refused `unknown`, `started`, `no_open_seat` or `throttled` |
| `POST /rooms/:code/act` | `RoomAction` | `{ ok: true }`, or refused with the rule that said no |
| `POST /frontline/join` | | `Briefing`, or refused `full` |
| `POST /frontline/:battle/deploy` | | `BattleTicket`, or refused `expired` |
| `GET /battles/:id/stream?after=<id>` | | battle stream for the caller's side |
| `POST /battles/:id/up` | `UpBatch` | `UpReply` |
| `POST /battles/:id/preview` | `PreviewQuery` | `PreviewAnswer` |
| `POST /dev/battle` | | `BattleTicket`. Local scaffolding from slice 08, deleted in slice 18. |

## Record framing

One framing for every stream. Little-endian, a 12-byte header so that `f32`
payloads stay aligned, then the payload padded to four bytes.

```
u32 payloadBytes | u32 id | u8 kind | u8 flags | u16 0 | payload
```

| Kind | Payload | Meaning |
|---|---|---|
| `open` | JSON `StreamOpen` | First record of every response |
| `publication` | binary | One tick of one side: `u32 tick`, 16 ASCII bytes of digest, `f32 stepMs`, then the publication's `f32` words |
| `status` | JSON | The battle's status changed (`running`, `finished`, `failed`) |
| `state` | JSON | The whole current `RoomList` or `RoomView` |
| `end` | JSON `{ reason }` | The server's last word on this stream |

`flags` bit 0 marks a keyframe: a publication that is a full snapshot.

Ids count up from 1 per stream source (one battle side, one room, the room
list). `open` and `end` records are not numbered: their id is 0. Chunk
boundaries mean nothing: a decoder must accept any split.

```ts
type StreamOpen = {
  resume: "continued" | "keyframe";
  layout?: string; tickHz?: number;      // battle streams only
};
```

## Resuming

The reader reconnects with `?after=<last id it handled>`.

- **The id is still in the ring:** `open` says `continued`, and the records
  after that id follow. Nothing is repeated and nothing is skipped.
- **It is not, or there is no `after`:** `open` says `keyframe` and the side's
  publisher is asked to resync. That reader is sent nothing until the next
  record flagged as a keyframe, which is the full snapshot; every reader
  already on that side receives the same snapshot in line. After `keyframe`
  the reader's numbering starts afresh from whatever comes next. A side is
  resynced at most once a second.
- **An id that is not the next one, without `keyframe`,** is the server
  breaking this rule. The reader fails; it does not reconnect.
- **An `end` record** is the last: the reader stops and does not reconnect.
  Until then a reader reconnects at once after a connection that delivered
  something, and backs off while connections deliver nothing.

State streams need no ring: on any open the server sends the current state.

The server never waits for a reader. A socket whose queue passes a bound is
closed, and that reader resumes like any other.

## Orders

```ts
type UpBatch = {
  seq: number;                       // counts up per player; a retry reuses it
  items: { id: number; order: Order; queued: boolean }[];
};
type UpReply =
  | { seq: number; results: { id: number; ack: CommandAck }[] }
  | { seq: number; refused: "not_seated" | "rate" };
```

- A page has one batch in flight. Orders issued meanwhile wait and leave
  together in the next batch, in the order they were given.
- A batch whose reply does not arrive is sent again with the same `seq`. The
  server keeps each seat's last `seq` and reply, and answers a repeat from that
  record without applying it twice.
- The battle stamps side, player and sequence. The page names none of them.
- Orders past the simulation's per-player rate are refused `rate` before they
  reach the battle, so a hostile page cannot grow the order log.

## Previews

```ts
type PreviewQuery =
  | { kind: "purchase"; variant: string; destination: [number, number] }
  | { kind: "move"; move: MovePreviewRequest }
  | { kind: "building"; building: BuildingPreviewRequest };
type PreviewAnswer =
  | { purchase: PurchasePlacement }
  | { moves: MoveDestination[] }
  | { building: BuildingPlacement | null }
  | { dropped: true };
```

Newest wins and nothing is retried. The server answers about five a second per
player and drops the rest.

## Build identity

```ts
type BuildIdentity = { commit: string; sim: string };  // sim: SHA-256 of the served simulation file
```

The page sends its own in `/hello`. The server compares it with the
`build.json` it was deployed with. A page on any other build is told online
play is not available on it.

## Tickets

```ts
type BattleSetup = {            // what both ends feed the one request builder
  seats: { id: SeatId; side: "blue" | "red"; controller: ControllerKind }[];
  factions: Record<"blue" | "red", Faction>;
  map: MapChoice; battleSeed: number; lighting: string;
  rules: "skirmish" | "frontline";
};
type BattleTicket = {
  battle: string;               // the id in /battles/:id
  setup: BattleSetup;
  mapIdentity: string;          // the server's; the page refuses to start on a mismatch
  side: "blue" | "red";
  player: string;               // the battle's player id for this seat
};
```

A ticket carries the settings, not the prepared battle. The page builds the
map itself from the settings and checks its identity against the server's.
Nothing in a ticket, a `RoomView` or a `Briefing` names a browser id: that is
a player's credential, and other players see a callsign.

## Rooms

```ts
type RoomCode = string;         // "TANGO-4821": the id, the password and the link
type SeatId = string;           // minted once, never renumbered
type SeatView = {
  id: SeatId; side: "blue" | "red";
  occupant:
    | { kind: "open" }
    | { kind: "ai" }
    | { kind: "human"; callsign: string; ready: boolean; host: boolean };
};
type RoomView = {
  code: RoomCode; visibility: "public" | "private";
  seats: SeatView[];
  settings: { factions: Record<"blue" | "red", Faction>; map: MapChoice; lighting: string };
  you: SeatId;
  canStart: boolean;            // every human ready, a fighter on each side
  phase: { kind: "lobby" } | { kind: "battle"; ticket: BattleTicket };
};
type RoomList = {
  rooms: { code: RoomCode; host: string; map: MapChoice;
           blue: ("human" | "open" | "ai")[]; red: ("human" | "open" | "ai")[] }[];
};
type RoomAction =
  | { kind: "sit"; seat: SeatId }
  | { kind: "ready"; ready: boolean }
  | { kind: "leave" }
  | { kind: "add_seat"; side: "blue" | "red" }
  | { kind: "remove_seat"; seat: SeatId }
  | { kind: "set_seat"; seat: SeatId; to: "open" | "ai" }
  | { kind: "settings"; patch: Partial<RoomView["settings"]> }
  | { kind: "start" };
```

Only public rooms still in the lobby appear in `RoomList`. The last five
actions are the host's.

## Frontline

```ts
type Briefing = {
  battle: string; side: "blue" | "red"; seat: number;      // 1 to 4
  credits: number;
  scores: [number, number]; target: number; minutesIn: number;
  setup: BattleSetup; mapIdentity: string;                 // so the page can build the map behind the card
  roster: { side: "blue" | "red"; callsign: string | null }[];   // null: an AI
};
```

`join` reserves the seat for about a minute. `deploy` takes it: the seat's
army is handed over, the seat is reset, and the ticket comes back.
