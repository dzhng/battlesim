# 17 Room lobby screen

**Stage:** Rooms. **Needs:** 15. **Carries:** decisions 33, 39, 50, 66, P28.

## Contract

The room is the skirmish lobby with several humans in it. One lobby
component, two drivers: local state for offline skirmish, and a `RoomView`
plus actions for a room. A guest sees the settings and cannot change them.

## Seam

- `apps/battle-lab/src/lobby/Lobby.tsx` becomes that one component. It takes
  its seats, settings and callbacks from its driver and holds no copy of a
  room.
- The room's driver lives under `apps/battle-lab/src/online/`: it opens the
  room's stream and turns each control into a `RoomAction`.
- A human's banner shows their callsign, with "you" on your own and "host"
  on the host's. Today every human banner reads "You"
  (`apps/battle-lab/src/lobby/Lobby.tsx:303`).

Use [game-ui](../../../.agents/skills/game-ui/SKILL.md) before building it.

**Target:** screen 3 of
[lobby-mock.html](../visualizations/lobby-mock.html), as host and as guest.
What the user confirmed: Ready shows only when true; an open seat is dashed
and can be clicked to sit; the host can set a seat open or AI, remove it and
add one; "N VS M" counts only seats that will fight; the note that open seats
sit out; the code and a Copy link button in the header; no chat. The sides
stay left and right and the map picture keeps one orientation, whichever seat
you take.

**Slice variable:** the lobby's seats and controls with several humans. Out
of scope: the battle, the map picture's art.

## Run or see

Two browsers in one room. One moves seats and readies; the other sees it at
once. The host's Start lights when everyone is ready.

## Verification

1. A scene with a host, a guest on the other side, an open seat and an AI
   seat, captured from both players' browsers.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md)
   twice: against the mock's screen 3, and the offline skirmish lobby before
   against after, which should not have changed.
3. Use [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)
   as the last check, unprimed.
4. Show the user with
   [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). Wait about
   five minutes; if they are silent, decide on the evidence, record it in
   decisions.md, close the shots and continue.
5. Tests of behaviour: a guest's settings controls do nothing; moving to a
   seat on the other side does not turn the lobby; a removed player is back
   at the Multiplayer page with a line saying why.

## Must stay green

The offline skirmish lobby's scenes and tests.

## Delegated

Component split inside the lobby; wording.

## As built (2026-11-15)

- **One lobby, two drivers.** `apps/battle-lab/src/lobby/Lobby.tsx` draws
  what its driver gives it: `localDriver` for offline skirmish, and
  `online/roomDriver` for a room, which opens the room's stream and turns
  each control into a `RoomAction`. The view keeps no copy of a room: a
  control sends an action and the screen changes only when the server's
  state arrives.
- **A seat row in a room** shows the callsign with "you" and "host", Ready
  only when true, and the lane the seat will fight on. An open seat is
  dashed, has no lane number, and is clicked anywhere to sit.
- **The host chooses what holds an empty seat from one picker,** "Open" or
  "AI", the control the offline lobby already uses on its AI seats (asked
  for by the user on seeing the first cut, which had "Set AI" and "Set
  open" buttons beside a separate tag). The host can also remove a seat or
  a player and add a seat.
- **A guest** sees the settings under a "Set by the host" label and cannot
  change them.
- **The header** carries the room's code, Copy link (the code and nothing
  else) and Leave. Start follows the server's word that the room can start;
  after it the room says "Starting…" until slice 18.
- **The map picture** is the room's exact seed through the lobby's own
  preview, the same for every viewer and the same whichever seat you take.
  Its status is worded as the map's ("Map ready"), so it cannot be read as
  a player's Ready.
- **A removed player** is back at the Multiplayer page with "You are no
  longer in that room."; a closed room, "That room has closed."
- **Server:** resuming a room's stream never joins. A browser that is no
  longer a member is refused `removed`.

**Fresh eyes on the first cut** found, and a fix round corrected:

- The seat list numbered seats where the map numbers lanes, so with an open
  seat above a filled one the map marked the "wrong" lane empty. A seat's
  numeral is now the lane it will fight on, taken from the same owner the
  battle uses to close up seats, and an open seat has none.
- A guest could not tell the settings were not theirs.
- "Ready" beside the settings meant the map and read as a player's state.
- Room rows mixed casing, and a map type of "open" read as "this room is
  open": the map text now leads with a "MAP" label and uses the lobby's
  own labels.
- The Ready button was smaller and lower than its neighbours; Red's open
  row was not mirrored; the host's remove mark on a player's row did not
  say it removes the player; the "refused" capture was taken under the
  loading cover.

Checked by tests against a real local room service and by the `room-lobby`
scene from both browsers at once (63 checks: lanes agree with the map in
every arrangement, the three buttons share a height and baseline, Red's
open row mirrors Blue's). The offline skirmish lobby's scene, tests and
pictures are unchanged.

Left as they are: the "Starting…" stand-in (slice 18); the Multiplayer tile
wearing Skirmish's picture until it has art; small occupancy marks in the
room rows, which fresh eyes still find hard to scan.

