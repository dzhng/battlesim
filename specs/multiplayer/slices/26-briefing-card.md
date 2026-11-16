# 26 Briefing card and Play

**Stage:** Frontline. **Needs:** 25. **Carries:** decisions 24, 44, 45, 51.

## Contract

One button puts a player in a fight. Play shows a briefing card; behind it
the browser builds the map, which takes about 15 seconds for a large one;
Deploy lights when it is ready; then the player is in the battle with opening
credits and no units.

## Seam

- The Frontline band at the top of the Multiplayer page (slice 16), with its
  Play button and a line of live numbers from the server.
- The briefing card under `apps/battle-lab/src/online/`, drawn from the
  `Briefing` in [wire.md](../wire.md): the side that needs a commander, the
  faction, the seat, the credits, the score against the target, how long the
  battle has run, and who is fighting on each side, humans by callsign and
  AIs as a count.
- Deploy calls `deploy` and follows the ticket into the online battle route.
  A reservation that lapsed sends the player back to Play with a line saying
  why.

Use [game-ui](../../../.agents/skills/game-ui/SKILL.md) before building it.

**Target:** screen 4 of
[lobby-mock.html](../visualizations/lobby-mock.html) with the briefing
option, and the Frontline band on screen 2. The mock's 5,000-point target and
its "your units arrive by road 2" callout are placeholders: show the real
target, and keep a callout only if game-ui supports one.

**Slice variable:** the briefing card's content and reading order. Out of
scope: the battle behind it.

## Run or see

Multiplayer, Play, the card, Deploy, and a first purchase driving in.

## Verification

1. A scene of the card for a game well under way, and one for a game just
   created.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md)
   against a capture of the mock's screen 4, saved in
   `specs/multiplayer/assets/`. Judge structure and reading order.
3. Use [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)
   as the last check, unprimed.
4. Show the user with
   [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). Wait about
   five minutes; if they are silent, decide on the evidence, record it in
   decisions.md, close the shots and continue.
5. Tests of behaviour: Deploy is unavailable until the map is built; "servers
   are full" is shown when Play is refused; a lapsed reservation returns to
   Play.

## Must stay green

Slice 16's Multiplayer scenes, apart from the new band.

## Delegated

Wording; the live line's content.

## As built (2026-11-15)

Frontline is the leading band on Multiplayer. Its live games and active human
commanders come from the existing room-list stream. Play stays busy through
server admission, including map retries. Full capacity uses the room page's
existing refusal; failed generation has its own line. A forgotten callsign is
repaired through the same hello owner and join is retried once.

The briefing names teams “Your side” and “Enemy”, followed by their factions,
and reads assignment, credits, own and enemy scores against the real target,
then both rosters, then Deploy. New games say “Battle just begun”. There is no
road callout. Departed commanders driven by AI count as AI on the roster.

Exact-map preparation is shared with room battles. Deploy stays shut until it
succeeds and matches the server. The battlefield it built is handed to the
battle page in memory, so the battle does not build it twice; it is 5 to 8 MB,
too large for the history entry, and a reloaded page builds it again. A failure
keeps the card and offers Back. The shared battle view's cover remains through
the first snapshot and first frame. Back and Escape return the reservation;
chosen battle departures use the existing menu confirmation, send leave, and
return to Multiplayer. Idle returns there with an explanation on the band;
the existing result screen does too.

Checked at integration: the whole multiplayer domain (254 tests), and the
`frontline`, `multiplayer`, `online-battle` and `room-lobby` scenes on the
integrated tree.
