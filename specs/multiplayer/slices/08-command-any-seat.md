# 08 Command any seat

**Stage:** Foundation. **Needs:** 07. **Carries:** decision 14, P23; landmine 5
(the function, not the look). **This is the first playable checkpoint.**

## Contract

A battle page can be any seat on either side of a battle the server runs. Two
browsers, one commanding Blue and one commanding Red, play one battle against
a local server.

## Seam

- **`apps/battle-lab/src/routes/onlineBattle.tsx`** (new), at
  `/online/battle/:id`. From a `BattleTicket` it builds the world locally with
  the existing `prepareBattle`, refuses to start if its map identity differs
  from the ticket's, and hands the battle session a connection made by
  `remoteConnection`. `useBattleSession` already takes a side, a player and a
  prepared session (`apps/battle-lab/src/useBattleSession.ts:98`); it does
  not grow.
- **The opening camera** reads the commanding player's road, not the first
  human's (`crates/sim/src/entries.rs:134`).
- **`apps/battle-lab/src/battleLinks.ts`** accepts a human on Red and more
  than one human (it refuses both at line 181 today).
- **Scaffolding (P23):** `POST /dev/battle` on a server started with its dev
  flag makes one fixed 1v1 and gives each caller the next free seat's ticket.
  The page `/online/dev` calls it and opens the battle. Both are deleted in
  slice 18.

How the screen reads for a Red commander (colours, which way is up) is
slices 11 and 12. Here it only has to work.

## Run or see

`bun run server` with the dev flag, `bun run dev`, then open `/online/dev` in
two browsers. The first is Blue, the second Red. Each can buy, move and fight,
and sees only what its own side sees.

## Verification

- A browser scene with two pages: each gives one order, and each sees its own
  order take effect and the other's units only when in sight.
- A page whose map identity differs from the ticket's refuses before opening
  the stream.
- Pause, speed and replay export are absent from the online battle's screen.
- Use [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)
  on the Red commander's screen as the last check. Brief it only to report
  anything that does not work or cannot be read; colour and orientation are
  out of scope here and belong to slices 11 and 12.
- Show both screens to the user with
  [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This
  checkpoint does not block: after about five minutes with no reply, record
  what you saw in decisions.md, close the shots and continue.

## Must stay green

Single-player skirmish and its scenes. `useBattleSession.ts` no larger than
before.

## Delegated

Where the online flag enters the battle view; the dev page's look (it is
scaffolding).

## Feedback that would change this

Anything the user finds unplayable here is fixed here. Anything about how it
looks goes to slices 11 and 12.

## As built

- **`/online/dev`** asks the local server for a ticket and opens
  **`/online/battle/:id`** with it. The battle page generates the map
  locally from the ticket's setup, compares its hash to the ticket's, and
  only then makes its connection. A page opened at the battle address
  without a ticket is refused.
- **`POST /dev/battle`** exists only on a server started with `--dev`
  (`bun run server --dev`). The first caller is Blue, the second Red, a
  third is refused, and a returning browser gets its own ticket again. The
  battle is created on the first call. The environment-variable seating
  that slice 07 used is deleted.
- **The opening camera** stands on the commanding player's own road. No
  simulation change was needed: the page already computes each player's
  road.
- **`battleLinks`** already accepted a human on Red and more than one human
  (slice 04 removed the refusal). Tests now cover both.
- **The online client asks only for its side and its connection,** and
  opens with a `connect` request where a local one sends `init`. The
  session gives every view the seat's actions and, apart, the local
  authority's controls (pause, speed, restart), which an online battle does
  not have. The pause menu online is titled "Menu" and pauses nothing.

Decided here where the plan was silent:

- The dev page talks to `http://127.0.0.1:8787`; a development build takes
  another local address from `?server=`.
- The browser id is 16 random bytes in hex, kept in the tab's session
  storage. It travels only in the `X-Player` header. Slice 09 owns where it
  lives for longer.
- The ticket and the server's address travel in the router's history state,
  never in the address bar.

To play it by hand, from the repository root in two terminals:

```sh
bun run server --dev
bun run --cwd web dev --host 127.0.0.1 --port 5173
```

Then open `http://127.0.0.1:5173/online/dev` in two separate tabs or
browsers, Blue first. A duplicated tab copies the first tab's id and gets
the same seat.

## Checkpoint (2026-11-15)

The scene `online-battle` starts a real server, opens two pages, and plays:
each commander buys a unit through the reinforcements picker, starts, and
orders it forward with a right-click. It passed twice running at
integration, 21 checks each time:

- each page's own units belong to its seat and the enemy is absent until
  the two armies meet, then appears on ground that page can see;
- a page given a ticket with another map hash refuses with no stream opened;
- no pause, speed or replay export can be found on either screen, and the
  battle keeps ticking with the menu open;
- the two commanders open on different roads.

One of its checks was wrong as delivered and failed here: it waited for each
page to see the enemy, then looked again a moment later, by which time the
fight had moved on. It now takes its proof in the frame where it holds.

Single-player is unchanged: the `lobby`, `unit-roster` and `ui` scenes pass
with every approved picture matching.

**An unprimed look at the Red commander's screen** found nothing broken by
this slice: no error or loading cover, the unit selected and its callout
readable, the command bar and reinforcements panel present. Everything it
did report belongs to the battle screen as single-player has it too (the
objective label drawn over a unit standing on the objective, icon-only
command buttons, the unlabelled score readouts), plus a faint frame-rate
readout in a corner. Those are noted here for the screen's owner and are
not this slice's to change. Colour and orientation were out of scope
(slices 11 and 12): the Red commander's own unit wears an orange ring and
Blue-style text.

Known rough edges, by design until later slices: motion is uneven (playout
is slice 10), the first frame after joining can take up to a second, and a
page reloaded mid-battle rejoins from the dev page, not from its address.

