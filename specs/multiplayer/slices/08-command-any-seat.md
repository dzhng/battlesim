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
