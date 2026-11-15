# 04 Seats and setup the server can run

**Stage:** Foundation. **Needs:** nothing. **Carries:** decisions 39, 41 to 43,
58, P1, P15, P28; landmine 6.

## Contract

- A seat has an id that never moves. It holds a human, an AI, or nothing.
- A lobby is a list of seats plus its settings. Several humans may sit, on
  either side.
- One function turns a lobby into the battle's player list: open seats are
  dropped and the rest close up, so roads follow the filled seats in order.
- One function turns a lobby into a battle request, on the page and on the
  server alike.

Today seat ids are positions (`apps/battle-lab/src/skirmishSetup.ts:36`), so
removing a seat renumbers the rest. The lobby model refuses a second human and
any human on Red, and reaches the renderer and bundler-only imports through
`battleLinks.ts` and `skirmishSetup.ts`.

## Seam

New package `packages/skirmish-setup/src/`, relative imports only, no DOM and
no Node:

```ts
type SeatId = string;                                  // minted once
type Occupant = { kind: "human"; who: string } | { kind: "ai" } | { kind: "open" };
interface Seat { id: SeatId; side: "blue" | "red"; occupant: Occupant }
interface LobbyState {
  seats: Seat[]; nextSeat: number;                     // the lobby mints its own ids
  factions: Record<"blue" | "red", Faction>;
  map: MapChoice; battleSeed: number; lighting: string;
}
function battlePlayers(seats: readonly Seat[]):
  { players: Player[]; playerOf: Record<SeatId, string> };
function battleRequest(lobby: LobbyState, data: SetupData): PrepareBattleRequest;
```

- `seats.ts` and `lobby.ts` move from `apps/battle-lab/src/lobby/model.ts` and
  `skirmishSetup.ts`: seat rules, the size floor, faction rules.
- `SetupData` is the game data `skirmishSetup.ts` imports today with `?raw`
  and `@fixtures`. The page passes its bundled copies; the server will read
  the same files from disk (slice 06).
- Building a link (`toUrl`, `battleHref`) stays in the app. The model no
  longer imports it.
- The one-Blue-human rule and the Red refusals in the model are deleted on
  purpose. The offline skirmish page seats its one human where the player
  puts them.

## Run or see

The skirmish lobby looks and plays as before. Removing a seat leaves its
neighbours' ids alone.

## Verification

- Existing lobby tests, re-pointed at the package.
- New: removing a seat changes no other seat's id; open seats are absent from
  `battlePlayers` and do not move the map's size or road count; the players
  close up in seat order; a side with no human or AI is refused.
- New `web/tests/multiplayer/serverGraph.test.ts`: bundling the package's
  entry for Node with no alias configured succeeds. It fails on a bundler
  alias, `?raw` or `?url` anywhere in the graph. Later slices add the server's
  entry to the same test.

## Must stay green

Skirmish digests for the same lobby settings. `web/tests/battleLinks.test.ts`.

## Delegated

The file split inside the package; how seat ids are minted.

## Feedback that would change this

If the user wants the offline lobby to keep its human on Blue, that becomes a
rule of the skirmish page, not of the model.

## Seen in a browser (2026-11-15)

After slices 02 and 03 and a merge of staging, the `lobby` scene passes all
22 of its checks (presets, Add and Delete, taking another Blue seat, the
size floor, Reroll, Deploy preparing the request the preview admitted) and
the `ui` scene's picture of the lobby matches its approved one with no pixel
differing. Both need the menu's large files fetched first: runtime art and
audio, and `assets/third-party`.

