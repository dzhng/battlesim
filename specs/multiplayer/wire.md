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
  the player credential. The private lobby-to-battle control surface uses a
  separate per-battle key in `X-Battle-Key`; players never receive it. Player
  routes derive the seat from `X-Player` and ignore any side, player or seat
  a body claims.
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
| `GET /build` | | `BuildIdentity` only; no credential and no player registration |
| `POST /hello` | `{ build: BuildIdentity, callsign: string }` | `{ callsign }` (with `#2` on a clash), or refused `build` |
| `GET /rooms` | | state stream of `RoomList` |
| `POST /rooms` | `{ visibility: "public" \| "private" }` | `{ code: RoomCode }`, or refused `full`, `game_running` or `callsign` |
| `GET /rooms/:code` |  | state stream of `RoomView`; opening it joins the room. A newcomer is refused `unknown`, `started`, `no_open_seat` or `callsign` |
| `POST /rooms/:code/act` | `RoomAction` | `{ ok: true }`, or refused with the rule that said no |
| `POST /frontline/join` |  | `Briefing`, or refused `full` (no battle slot), `unavailable` (no battlefield could be made), `callsign` or `game_running` |
| `POST /frontline/:battle/deploy` | | `BattleTicket`, or refused `expired` |
| `POST /frontline/:battle/leave` | | `{ ok: true }`. The seat goes back to its AI. |
| `GET /battles/:id/stream?after=<id>` | | battle stream for the caller's side |
| `POST /battles/:id/up` | `UpBatch` | `UpReply` |
| `POST /battles/:id/preview` | `PreviewQuery` | `PreviewAnswer` |

## Private battle start

The lobby asks one battle server to try ordered candidates with authenticated
`POST /control/start`: `{ id: string, candidates: BattleSetup[] }`. Success is
`{ layout: string, tickHz: number, mapIdentity: string, candidate: number }`,
where `candidate` is the zero-based admitted index. No admitted candidate gives
the existing generic `unavailable` refusal. A second start is refused `started`.
Browser credentials and the battle key never appear in the reply. Player-facing
tickets and briefings carry only the admitted setup, with the existing shapes.

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
| `status` | JSON `{ status, slow }` | The battle's status changed (`running`, `finished`, `failed`), or it began or stopped running behind its clock (`slow`) |
| `state` | JSON | The whole current `RoomList` or `RoomView`; the private control stream uses events and its reconciliation snapshot |
| `end` | JSON `{ reason }` | The server's last word on this stream |

`flags` bit 0 marks a keyframe: a publication that is a full snapshot.

Ids count up from 1 per stream source (one battle side, one room, the room
list). `open` and `end` records are not numbered: their id is 0. Chunk
boundaries mean nothing: a decoder must accept any split.

```ts
type StreamOpen = {
  resume: "continued" | "keyframe";
  layout?: string; tickHz?: number;      // battle streams only
  status?: { status: "running" | "finished" | "failed"; slow: boolean };
                                         // a battle's status as last said, for a late reader
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

The private control stream uses the same framing and resume rule, with `state`
records for events and a keyframe snapshot of live connections and order
activity after a gap. It never contains a browser id or the battle key.

State streams need no ring: on any open the server sends the current state.

The server never waits for a reader. A socket whose queue passes a bound is
closed, and that reader resumes like any other.

## Orders

```ts
type UpBatch = {
  seq: number;                       // counts up from the page's clock, so a reloaded page is later; a retry reuses it
  items: { id: number; order: Order; queued: boolean }[];
};
type UpReply =
  | { seq: number; results: { id: number; ack: CommandAck }[] }
  | { seq: number; refused: "not_seated" | "rate" | "invalid" };
```

- A page has one batch in flight. Orders issued meanwhile wait and leave
  together in the next batch, in the order they were given.
- A batch whose reply does not arrive is sent again with the same `seq`. The
  server keeps each seat's last `seq` and reply, and answers a repeat from that
  record without applying it twice.
- The battle stamps side, player and sequence. The page names none of them.
- A batch beyond the simulation's per-player allowance for one window is
  `invalid`; a batch within that bound but beyond its occupant's remaining
  allowance is `rate`. Both are refused before reaching the battle.
- HTTP input bounds each order's encoded size and unit references (including
  `collect` squads), before any item in the batch reaches the battle. The
  bounds live with `apps/game-server/src/http/input.ts`; ordinary whole-army
  selections fit. An offending batch is `invalid`, without spending allowance.
- The sequence cache and transport allowances belong to a seat's occupant.
  Reconnecting the same browser keeps them; seating a different browser resets
  them, even if that browser has not yet sent an order.

## Previews

```ts
type PreviewQuery =
  | { kind: "purchase"; variant: string; carrier?: string; destination: [number, number] }   // carrier: the squad bought aboard it
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
type BuildIdentity = { sim: string; data: string; wire: string };
```

The page sends its own in `/hello`. The server refuses `build` exactly when
one of these three differs from its deployed `build.json`. A different source
commit with equal hashes is admitted. `sim` hashes the module bytes the page
actually loads. `data` covers the documents both ends build and field a battle
from, including JSON embedded in the bundle; `wire` covers multiplayer sources.
The [build-time owner](../../packages/multiplayer/buildIdentity.ts) defines the
file sets and hashes sorted paths, byte lengths and file bytes for the latter
two. Development and production builds use it, as does the server bundle.

The site's `build.json` also records its full source commit; the menu retains
its app-commit line. Neither is compared for admission. The credential-free
`GET /build` projects only the three hashes, so release preparation can ask if
a live lobby admits a site without changing player state.

## Tickets

```ts
type BattleSetup = {            // what both ends feed the one request builder
  seats: { player: string; side: "blue" | "red"; controller: ControllerKind }[];  // the battle's players, in order
  factions: Record<"blue" | "red", Faction>;
  map: MapChoice; battleSeed: number; lighting: string;
  rules: "skirmish" | "frontline";
};
type BattleTicket = {
  address?: string;             // battle origin, omitted when it is the lobby origin
  battle: string;               // the id in /battles/:id
  setup: BattleSetup;
  mapIdentity: string;          // the server's map hash; the page refuses to start on a mismatch
  side: "blue" | "red";
  player: string;               // the battle's player id for this seat
  callsigns: Record<string, string>;  // public names, keyed by battle player id
};
```

The lobby serves the room and Frontline endpoints. The battle server alone
serves `/battles/:id/{stream,up,preview}`. The page uses `ticket.address` for
its battle link, falling back to the lobby address only when the field is
absent. Room/list presence, leave and deploy still use the lobby address.

A ticket carries the settings, not the prepared battle. The page builds the
map itself from the settings and checks its identity against the server's.
Nothing in a ticket, a `RoomView` or a `Briefing` names a browser id: that is
a player's credential, and other players see a callsign.

## Rooms

```ts
type RoomCode = string;         // six unambiguous letters/digits: the id and shareable link
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
  phase:
    | { kind: "lobby" }
    | { kind: "starting" }       // the host started; the server is building the battle
    | { kind: "battle"; ticket: BattleTicket };
};
type RoomList = {
  frontline: { battles: number; commanders: number }; // running Frontline games and their seated humans
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
  | { kind: "settings"; patch: RoomSettingsPatch }
  | { kind: "start" };
type RoomSettingsPatch = {      // exactly what a host may set; anything else is refused
  map?: { type?: MapType; size?: MapSize; region?: string | null; seed?: string };
  factions?: Partial<Record<"blue" | "red", Faction>>;
  lighting?: string;
};
```

Room codes are not secrets. `normalizeRoomCode` in the shared room model
removes whitespace and ASCII dashes anywhere, then uppercases. The page and
server use it; links carry the canonical code. No wrong guesses are counted,
and no refusal conceals whether a room exists. The alphabet is data in that
same owner; old word-and-digits links have no compatibility path.

A device may have one game of its own making: a room it created or a Frontline
game whose initial Play created it. `game_running` says that place is already
held; the page says "You already have a game running." Joining existing games
is unrestricted. Ownership begins before preparation and lasts until the game
is removed, including when another player inherits hosting or keeps playing.
All removal paths and failed preparations release it.

The lobby's configured `clientAddressHeader` names the trusted proxy header.
Room and Frontline requests without it are refused `address`; without a
configured header, the device is `X-Player`, never the socket address. Device
identity and game ownership each have one shared server owner and appear in
no public view or response.

A host's settings come from the network. Each value is checked against the
game's own lists and applied through the lobby's own rules, so a size below
the floor is raised to it and an unknown value is refused. A map seed must
be accepted by the map generator's `canonicalSeed`; the room stores its
canonical decimal value. Other values are `unknown map seed`.

The room list publishes only when its public snapshot changes, including
either Frontline count. Ready, identical settings, and private-room changes
send no list record. A new subscriber still receives the current snapshot.
Reservations in an existing game are not seated humans; the first commander
of a new game is seated from tick zero. A departed seat running on AI does
not count as a human.

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
