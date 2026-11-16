# 24 Frontline placement

**Stage:** Frontline. **Needs:** 05. **Carries:** decisions 21, 24 to 26, 30,
61, P18 to P20.

## Contract

Where the Play button puts a player, and when a seat comes free, as functions
with no network and no clock of their own.

## Seam

New `packages/multiplayer/src/frontline.ts`:

```ts
interface GameView {                    // what the server knows of one game
  battle: string; scores: [number, number]; target: number;
  seats: { side: "blue" | "red"; number: number;          // 1 to 5
           held: { who: string; lastOrderAt: number; goneSince: number | null } | null;
           reservedUntil: number | null }[];
  granted: string[];                    // players who have had opening credits here
}
function place(games: GameView[], who: string, now: number, random: () => number,
               slotFree: boolean):                    // the server's knowledge, not the model's
  | { resume: string; side: "blue" | "red"; seat: number }     // still holds a seat: no seat events
  | { join: string; side: "blue" | "red"; seat: number; opening: boolean }
  | { create: true }
  | { full: true };
function seatsToFree(game: GameView, now: number): { side: "blue" | "red"; seat: number; why: "idle" | "gone" }[];
function newGame(random: () => number): BattleSetup;      // wire.md
```

The rules it holds:

- A candidate game is one whose leader is under 90% of the target and whose
  losing side has a seat from 1 to 4 that an AI holds and nobody has
  reserved. The losing side has the lower score; level scores go to the side
  with fewer humans, then a coin toss (P19).
- Among candidates, the one with the most humans. With none, a new game, or
  `full` when no battle slot is free.
- The seat is the lowest-numbered free one. Seat 5 is never offered.
- `opening` is true only the first time this player joins this game.
- A player who still holds a seat resumes it. That is a different answer
  from a join, because a join fires the hand-over and the reset and a resume
  fires neither.
- A new game has five seats a side, all AI, two different factions drawn at
  random, a random map kind and region, and the smallest size that seats five
  a side (P18). The player who caused it takes a seat from tick zero, with no
  seat event.
- A held seat comes free after about three minutes with no order, or about a
  minute after its player's stream went.

## Run or see

Tests only.

## Verification

`web/tests/multiplayer/frontline.test.ts`, one test per rule, phrased as
where a player lands: on the losing side; in the fullest game; never in seat
5; never in a game past 90%; in a new game when none fits; told the servers
are full when no slot is free; without a second opening grant on rejoining;
out of their seat after idling.

No test names the target, the timings or a faction.

## Must stay green

`serverGraph.test.ts`.

## Delegated

The timings' exact values near those above.
