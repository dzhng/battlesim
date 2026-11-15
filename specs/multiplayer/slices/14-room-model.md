# 14 Room model

**Stage:** Rooms. **Needs:** 04, 05. **Carries:** decisions 33, 35, 39, 41 to
43, 48, P12, P13.

## Contract

Every rule of a room as a function with no network, no clock of its own and
no storage.

## Seam

New `packages/multiplayer/src/room.ts`:

```ts
interface Room {
  code: RoomCode; visibility: "public" | "private";
  lobby: LobbyState;                                  // slice 04
  members: { who: string; callsign: string; seat: SeatId; ready: boolean; since: number }[];
  host: string;
  phase: "lobby" | "battle";
}
function createRoom(host: Member, visibility: Room["visibility"], code: RoomCode, now: number): Room;
function roomReduce(room: Room, by: string, action: RoomAction | { kind: "join"; callsign: string }, now: number):
  Room | { refused: string };
function canStart(room: Room): boolean;
function roomView(room: Room, who: string): RoomView;  // wire.md
function roomCode(random: () => number): RoomCode;     // "TANGO-4821"
```

The rules it holds:

- Joining seats you in the first open seat, on the side with fewer humans. No
  open seat refuses you. A started room refuses you.
- A player moves to any open seat on either side. Moving clears their ready
  mark.
- The host alone changes the map, size, region, lighting and factions, adds
  and removes seats, and sets a seat open or AI. Removing a seat someone sits
  in removes that player from the room. Up to five seats a side.
- Start needs every human ready and a human or an AI on each side.
- When the host leaves, the human who has been in the room longest hosts.
- An open seat is not in the battle and does not move the map's size or road
  count; seats close up at the start (slice 04's `battlePlayers`).

A seat's `who` is the player's browser id, which is their only credential.
It never leaves the server: `roomView` shows a callsign, and the
`BattleSetup` in a ticket carries each seat's controller and nothing about
who holds it. One function here turns a room into its `BattleSetup`, and one
turns a `BattleSetup` back into a lobby for the shared request builder, so
the page and the server build the same battle.

## Run or see

Tests only.

## Verification

`web/tests/multiplayer/room.test.ts`, one test per moment above, each
phrased as what a player can or cannot do: a guest cannot change settings or
start; the host cannot start while a human is not ready or a side is empty;
removing a seat leaves every other seat's id alone; the longest-present human
becomes host; a full room and a started room refuse a joiner; a code has the
`WORD-1234` shape.

## Must stay green

`serverGraph.test.ts`. Nothing under `crates/`.

## Delegated

The code's word list; internal shape of `Room`.

## Feedback that would change this

P12: if the user wants people in a room without a seat, `members` gains an
unseated state and the lobby screen a place to show them.
