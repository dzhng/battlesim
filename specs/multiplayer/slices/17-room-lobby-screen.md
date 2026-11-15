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
