# 18 Room battle

**Stage:** Rooms. **Needs:** 08, 15, 17. **Carries:** decisions 35, 37, 38, 39,
P7, P8, P14, P23.

## Contract

A room becomes a battle, survives a player dropping, and ends cleanly.

- **Start:** the host starts; the server makes the battle from the room's
  seats (open ones dropped, the rest closed up), and every member's
  `RoomView` turns to the battle phase carrying that member's ticket. The
  room leaves the list. Nobody can join it now.
- **Drop:** when the presence module says a player is gone, their seat is
  run by its AI, with the army it has.
- **Return:** the same browser id opening the room again gets its own seat
  back, at any point while the battle runs. Nobody else can take it.
- **End:** the result shows on each page from its final observation; everyone
  returns to the menu; the room and its battle are deleted (P14). A room
  whose humans have all been gone for a minute is deleted with its battle.

## Seam

- `apps/game-server/src/rooms.ts` calls `createBattleHost` with the setup
  from slice 04's functions, and maps each member to their battle player.
- The page's room driver follows the phase into the online battle route
  (slice 08) and back to the menu at the end.
- The existing preparation phase covers loading: it runs 60 seconds and ends
  early once every human is ready. A controller change does not touch a
  seat's ready flag; confirm that a dropped player's AI-run seat does not
  hold the phase open.
- **Delete the scaffolding:** `POST /dev/battle`, the `/online/dev` page and
  the server's dev flag go in this slice. Slice 08's two-seat scene now
  creates a room through the API.

This slice changes nothing under `crates/`. A room battle uses only what the
simulation already does: a seat's controller changing between a human and an
AI, which already replays exactly.

## Run or see

Two browsers: create, join, ready, start, fight. Close one tab; its units
keep fighting under the AI. Reopen the link; the seat is back. Play to the
end; both see the result and return to the menu.

## Verification

`web/tests/multiplayer/roomBattle.test.ts`, at the service seam:

- A started room has left the list and refuses a joiner.
- An open seat is not a player in the battle.
- After the grace period a dropped player's units receive orders from the AI.
- The dropped player returns to their own seat; another player cannot.
- The battle's replay, with its controller changes, ends on the same digest.
- After the end, the room and the battle are gone and their slot is free.

Then the two-browser scene from slice 08, now through a room.

## Must stay green

Every existing digest. Nothing under `crates/` changed.

## Delegated

The result screen's wording for an online match.

## Feedback that would change this

P14: if the user wants a rematch, the end returns everyone to the room's
lobby instead of the menu.

## As built (2026-11-15)

- **Start.** When the room service accepts the host's Start it builds the
  battle from the model's own setup (open seats dropped, the rest closed
  up) and the room turns to `starting`: off the list, closed to newcomers.
  When the battle's host is ready the service records the battle and each
  member's view carries that member's own ticket. The page says "Preparing
  the battlefield… This can take a few seconds." in between.
- **Seats.** A room's battle is served at `/battles/:id` like any battle.
  The room service answers who sits where, from the room itself.
- **Drop and return.** The one presence module decides. After the grace a
  gone member's seat is given to its AI with the army it has; they stay a
  member. The same browser opening the room again gets its own ticket and
  its seat back. Nobody else can join or take it. A battle stream counts as
  being there, and so does the room stream the battle page keeps open
  while it builds its map.
- **End.** A finished battle, a crashed one, and a room with nobody
  connected for a minute each dispose the battle, delete the room and free
  its slot. The room's stream ends with `battle_finished`,
  `battle_failed` or `room_deleted`. Each page keeps its last observation
  and shows the result, with a "Return to Multiplayer" button.
- **A crash.** Players' streams are told only `crashed`; the page says "The
  battle could not continue. The room has closed." The reason goes to the
  server's log, since an error's text may quote what a player sent.
- **The result screen online** shows callsigns on players' rows and in the
  timeline. A ticket carries the callsigns, keyed by battle player.
- **Playout at the end:** once the stream has said its last word the
  connection sends nothing more and plays out every frame it holds, so the
  final observation is the battle's last.
- **Deleted (P23):** `POST /dev/battle`, the fixed dev battle, the
  `/online/dev` page, the server's `--dev` flag and the deploy command's use
  of it. `bun run server` serves rooms with no flag.

To play it by hand, from the repository root in two terminals:

```sh
bun run server
bun run --cwd web dev --host 127.0.0.1 --port 5173
```

Open `http://127.0.0.1:5173/multiplayer` in two separate browser profiles.
Create a room in one and open its copied link in the other; both press
Ready; the host presses Start. Close one page for more than five seconds
and the other keeps playing against its AI; reopen the room's link in the
same profile to take the army back.

Checked at the service seam with real battles on a test clock
(`web/tests/multiplayer/roomBattle.test.ts`): tickets are each member's
own; open seats are not players; the AI orders a dropped player's units
only after the grace; the returning player's next order is applied and a
stranger is refused; a dropped seat does not hold preparation open; the
replay with its controller changes matches at every tick; a victory, a
crash, a failed map and an empty room each free the slot. The `online-battle`
scene now goes through a room from two browsers and plays to a real
victory under the ordinary rules: one side takes every objective.

**An unprimed look at both commanders' screens** (after a drop, after the
return, and at the result) found nothing clipped, overlapping or broken,
and the two result screens telling one story from two sides: victory
against defeat, each viewer's own side first and in their own colour, the
same tallies and timeline mirrored. Two things it could not find are worth
the user's attention, and neither is built:

- **Nothing tells a player that their opponent dropped or came back.** The
  battle carries on against the seat's AI in silence, and the result does
  not mention it.
- **The result does not say why the battle ended.** A match won by holding
  every objective stops at whatever the score was (73 to 48 here, against a
  target of 1,000). This is the offline result screen too.

The rest of what it listed is the result screen and battle HUD as
single-player has them: small dim column labels, a ragged name column in
the timeline, unlabelled score counters, icon-only command buttons.

