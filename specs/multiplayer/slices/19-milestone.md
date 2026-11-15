# 19 Milestone: a room battle across the internet

**Stage:** Rooms. **Needs:** 09 to 12, 16 to 18. **Carries:** decision 53.
**Judges:** O6.

## Contract

Two people on different networks create a room, play a battle through the
deployed server, and finish it. This is the user's own definition of "it
works".

## Steps

1. Run [review](../../../.agents/skills/review/SKILL.md) on what is going
   out.
2. **Ask the user** before promoting the site and before deploying the
   server: promotion ships everything on `staging`, and the server becomes
   reachable by anyone. Then promote, and run `bun run deploy:server` against
   the promoted site.
3. Prove it yourself first, through the public address: two browsers, a
   private room, a full battle. Check the built site, not only the inputs: a
   live battle loads, and a page on an older staging build is told online
   play is not available.
4. Write the run sheet for the user in this file under "Run sheet": the link,
   what to click, and the three things to watch for (a five-minute stream cut
   should be invisible; a closed tab should hand the army to the AI and give
   it back; orders should feel like about a third of a second).
5. Hand the run sheet to the user. Their battle with a friend is the
   sign-off, and their verdict on the feel closes O6.

This step does not block the build. Start slice 20 once step 3 passes.

## What the verdict changes

- **Orders feel fine:** O6 closes.
- **Orders feel too slow:** the host swap of decision 8 comes forward, which
  changes `apps/game-server/deploy/` and the address file. If previews are
  the complaint and orders are not, the deferred "previews in the browser"
  plan opens instead.
- **Something broke:** fix it in the slice that owns it, and run step 3
  again.

## Verification

Step 3's battle, recorded: the room code, both players' sides, the final
digest, the stream cuts survived, the measured order round trip.

## Run sheet

Not written yet.

## Results

Not run yet.
