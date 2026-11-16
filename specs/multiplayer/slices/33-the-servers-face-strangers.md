# 33 Six-character rooms, one game a device

**Stage:** Hosting. **Needs:** 31. **Carries:** decision 68. **Changes**
decision 14 and P25. **Blocks** putting the lobby's address in the site.

## Why

Before the site points at the lobby, a reader with no history in this plan
read the two servers as an attacker would (2026-11-16) and found seven
ways a stranger could cost the owner money or spoil other people's games.
The user's ruling the same day: this is an alpha, not an enterprise app.
Do not build a defence for each. One simple limit, the plain bugs fixed,
and private rooms made easier to join, not harder to find.

## Contract

- **A room's code is six characters,** from letters and digits that cannot
  be mistaken for one another, typed in any case. It is the room's id and
  its link, and it is not a secret: nothing counts wrong guesses and
  nothing hides whether a room exists. The throttle on guesses goes, with
  its refusal.
- **One game a device.** A device may have one game of its own making
  running at a time: a room it hosts, or a Frontline game it started.
  Asking for a second is refused with a line that says so. A device is the
  client's address as the host's proxy states it (`x-real-ip` on the
  public route, read on 2026-11-16; a caller's own value for it does not
  survive), and the browser id where there is no proxy. Joining somebody
  else's room or game is never limited by this.
- **Three plain bugs are fixed:**
  - A room's map seed is a seed: anything else is refused. (Any text up to
    the body limit was accepted and copied into the public list.)
  - An order that names more units, or is larger, than a seat could mean
    is refused before the battle sees it. (A stream of huge orders filled
    the battle's memory and ended it for everyone.)
  - A seat's place in its order sequence belongs to its occupant and is
    new for the next one. (A player could leave a seat no later player
    could give an order from.)
- **A list that has not changed is not sent again.** Every room action
  sent the whole public list to every reader.

## Left as it is, by the user's ruling

Counting or limiting streams and connections; where room codes' randomness
comes from; a client reading both sides with two ids; the order queue's
length; the replay keeping refused orders. Named so nobody rediscovers
them as news.

## Seam

`packages/multiplayer/src/room.ts` (the code, the seed's check) and
[wire.md](../wire.md); `apps/game-server/src/rooms.ts`, `http/rooms.ts`,
`frontline.ts`, `http/input.ts`, `http/seatTransport.ts`; the Multiplayer page's
join-by-code field and its refusal lines.

## Run or see

Create a private room: its code is six characters. Type it in lower case
on another browser and you are in. Create a second room from the same
device and you are told you already have one.

## Verification

1. A code typed in another case, or with spaces around it, joins. Many
   wrong codes in a row refuse nobody afterwards.
2. One game a device: a second room, a Frontline start while hosting, and
   a room while a Frontline game of one's own runs are each refused; a
   second device is not; joining is not; the place frees when the game
   ends.
3. Each of the three bugs as its attack: accepted before, refused after,
   and the battle or seat still works for the next player.
4. A room action that changes nothing in the list sends no list record.
5. A picture of the join field and the refusal line, with an unprimed
   critique.

## Must stay green

The multiplayer domain; `multiplayer`, `room-lobby`, `online-battle`.

## Delegated

The code's alphabet; the bounds on an order, as data beside their owner;
the refusal's name and wording.

## As built (2026-11-16)

The shared room model owns six-character generation and typed-code
normalisation; page and server both consume it. Codes are shareable ids, with
no guessing counter, timer or refusal. The alphabet avoids the common
letter/digit lookalikes; no old-link compatibility remains.

HTTP IO owns device identity from the configured address header or browser id.
Shared online state owns one creation claim per device. Rooms claim on creation;
Frontline claims only when Play must make a game, before preparation awaits.
Joining and resuming do not claim. The creating device keeps its claim through
host succession and departure until removal; finish, crash, empty-game cleanup,
timeouts and failed preparation release it. `game_running` reads "You already
have a game running." on either Multiplayer action.

Room settings use the map generator's canonical seed admission. HTTP input
bounds order bytes and unit references, including collect squads; the seat
transport uses the existing per-window allowance to bound batch length. An
invalid batch never reaches the battle. A new seat occupant gets a fresh
sequence cache and allowances; a reconnecting occupant keeps theirs. The battle
and its simulation rules are unchanged.

The room-list publisher appends only changed public snapshots. Room-only facts,
identical settings and private-room actions do not republish the list; opening
a stream still supplies its current state.

Verification receipts and remaining visual gates are in the ignored
`throwaway/multiplayer/slice33-report.md`. The domain run had 318 passes and one preparation timeout; that test passed
alone with its watchdog unchanged. Final focused checks passed. GPU scenes
were not started because the shared GPU lock was already held; it was neither
waited on nor changed. Isolated real-component captures of the field and both
refusal locations were inspected at 1280×720 and received an unprimed critique;
these do not substitute for the pending GPU scenes.
