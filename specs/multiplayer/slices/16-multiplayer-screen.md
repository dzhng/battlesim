# 16 Multiplayer screen

**Stage:** Rooms. **Needs:** 15. **Carries:** decisions 44, 46 to 48, 50.

## Contract

A Multiplayer tile on the main menu, above Skirmish, opening a page with the
callsign in its header, the list of public rooms, Create room (public or
private), and Join private room by code.

The Frontline band at the top of this page in the mock arrives with slice 26.
Until then the page holds rooms only, with no placeholder.

## Seam

- `apps/battle-lab/src/menu/pages.ts` gains the Multiplayer page; the tile
  sits above Skirmish in `Home.tsx`.
- A new page component under `apps/battle-lab/src/online/`, built from the
  menu's existing shell and components. It holds no room state of its own: it
  draws the `RoomList` stream and sends requests.
- The callsign field edits `identity.ts` (slice 09) and re-sends `/hello`.
- A page refused by `/hello` shows that online play is not available on this
  build, in place of the list.

Use [game-ui](../../../.agents/skills/game-ui/SKILL.md) before building it.

**Target:** screens 1 and 2 of
[lobby-mock.html](../visualizations/lobby-mock.html), with the options the
user chose: one tile; a random callsign, editable in the header; a room row
that says host, map and seats; started rooms not listed; a code that looks
like `TANGO-4821`. Seat strips: solid for a player, dashed for open, dim for
AI.

**Slice variable:** the layout and content of the menu tile and the room
list. Out of scope: the Frontline band, the room lobby.

## Run or see

Menu, Multiplayer, a list that updates as rooms are created elsewhere, and
Create and Join each landing in a room.

## Verification

1. A scene with a few rooms in different states, captured at the game's
   usual sizes.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md)
   against captures of the mock's screens 1 and 2. Save those captures in
   `specs/multiplayer/assets/`. The mock is a sketch: judge structure and
   reading order, and let the game's own components win on detail.
3. Use [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)
   as the last check, unprimed.
4. Show the user with
   [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). Wait about
   five minutes; if they are silent, decide on the evidence, record it in
   decisions.md, close the shots and continue.
5. Tests of behaviour, not of markup: the list follows the stream; an empty
   list says so; a wrong code says so and stays on the page; "servers are
   full" is shown when creation is refused.

## Must stay green

The main menu's existing scenes, apart from the new tile.

## Delegated

Wording of the empty and error states; the row's exact layout.
