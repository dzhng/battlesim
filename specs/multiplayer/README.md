# Multiplayer: rooms and Frontline

**Status:** building, and blocked on disk space. Nothing is integrated yet. **Updated:** 2026-11-14.

Two online modes on one game server. **Rooms:** a host creates a public or
private room, which is the skirmish lobby with several humans on either side,
and starts a battle. **Frontline:** one button drops a player into a battle
already running, on the side that is losing; every seat is played by a human or
by its own AI.

## Next Agent Prompt

You are building online multiplayer from this plan, with Codex writing each
pass in its own checkout and you integrating: read every diff, rerun its
checks yourself, review, then commit.

**Blocked on disk (2026-11-14).** The machine ran out of space while this
checkout was building the simulation's tests, with about 3 GB free
afterwards and other sessions building too. A Rust commit needs the
pre-commit hook's workspace clippy, which needs a debug build this disk
cannot hold. Do not build Rust here until the user has freed space; check
`df -h .` first, and stop under 5 GB.

**Where things are:**

- **Slice 05 is done** and committed (`931f58e89`), reworked at integration:
  see part 4 of [decisions.md](decisions.md).
- **Slice 04 is done** and committed, after two repair rounds with Codex and
  one fix at integration: see part 4 of [decisions.md](decisions.md).
  **Owed:** nobody has looked at the skirmish lobby in a browser since. It
  should look and play exactly as before; capture it against its baseline
  the next time a scene run is possible, before slice 17 builds on it.
- **Slices 14 and 24** (the room model and Frontline placement, both pure
  TypeScript) are with Codex in this checkout, uncommitted.
- **Slices 02 and 03** came back from Codex as two commits on the local
  branch `t3/per-side-publisher-sim-orders` (`67844ee46`, `25dad0b20`).
  Slice 02 is half integrated: its changes and two fixes sit uncommitted in
  this checkout, with a copy at
  `throwaway/multiplayer/slice02-integration-wip.patch`. It is not ready;
  read "Found at integration" in [slice 02](slices/02-per-side-publisher.md)
  before touching it. Slice 03 has not been looked at beyond its diff.
- **Slice 01's** two-hour native run is finished and written up in the slice
  file: memory, not tick cost, is what limits a long battle. Its host half,
  O1, has not been started.

**Next, once there is space:** finish slice 02 as its file says, integrate
03, and hand out slice 06. Slices 13 and 20 need nothing first. Slice 14
(the room model) needs only 04 and 05 and no Rust build, so it can go out
now.

**Keep parallel work small.** One checkout per child costs about 1 GB before
any build, and a Rust build adds 1 to 2 GB. Run at most two children at once,
give slices that share no files to one child, one commit each, and delete a
child's build output as soon as its commits are safe on a branch.

**What Codex needs watching for.** Both defects in slice 02 hid behind edited
tests: one assertion was repinned to the new numbers and one was deleted with
a comment. Read every changed test for a weakened or removed assertion before
reading anything else.

1. Read [decisions.md](decisions.md) (every row is settled; do not reopen one)
   and [wire.md](wire.md) (the one wire contract). The walk that produced the
   decisions is [the map](visualizations/unknowns-map.html); external practice
   is in [research.md](research.md).
2. Start [slice 01](slices/01-probes.md) in the background: its two-hour run
   and the host probe need no product code. While it runs, build slices 02 to
   05, which depend on nothing and on each other in no order.
3. Work the ladder below in order after that, with
   [implement-spec](../../.agents/skills/implement-spec/SKILL.md). Each slice file names its seam,
   its proof, and what is left to you. Anything a slice leaves you to invent
   that it did not name as delegated is a gap: decide it, and record it in
   part 4 of [decisions.md](decisions.md).
4. Commit each slice when its proof passes. Update this section before you end
   a pass: status, the next pickup point, and the checklist.

**Warnings**

- `apps/battle-lab/src/useBattleSession.ts` is over the file-size budget. It
  may shrink; it may not grow.
- Promoting the site, deploying the server and making it public are
  outward-facing. Ask the user before each (slices 09 and 19).
- The Bento CLI on this machine is signed in to the account that owns the
  `armata` workspace. The probe app `mp-probe` is still registered there,
  private and idle; slice 01 reuses it and slice 27 deletes it.
- Bento is a temporary host. Only `apps/game-server/deploy/` and the address
  file may know it exists.

**Checklist**

Foundation
- [ ] [01 Host and endurance probes](slices/01-probes.md)
- [ ] [02 Per-side publisher](slices/02-per-side-publisher.md)
- [ ] [03 The sim stamps orders](slices/03-sim-stamps-orders.md)
- [x] [04 Seats and setup the server can run](slices/04-seats-and-setup.md)
- [x] [05 Record stream and seat link](slices/05-record-stream.md)
- [ ] [06 Battle host](slices/06-battle-host.md)
- [ ] [07 HTTP transport](slices/07-http-transport.md)
- [ ] [08 Command any seat](slices/08-command-any-seat.md) (first playable checkpoint)
- [ ] [09 Identity, build check, deploy](slices/09-identity-build-deploy.md)
- [ ] [10 Playout](slices/10-playout.md)
- [ ] [11 Yours is blue](slices/11-yours-is-blue.md)
- [ ] [12 Your side at the bottom](slices/12-your-side-at-the-bottom.md)
- [ ] [13 Native and WebAssembly parity](slices/13-native-wasm-parity.md)

Rooms
- [ ] [14 Room model](slices/14-room-model.md)
- [ ] [15 Room service](slices/15-room-service.md)
- [ ] [16 Multiplayer screen](slices/16-multiplayer-screen.md)
- [ ] [17 Room lobby screen](slices/17-room-lobby-screen.md)
- [ ] [18 Room battle](slices/18-room-battle.md)
- [ ] [19 Milestone: a room battle across the internet](slices/19-milestone.md)

Frontline
- [ ] [20 An AI inherits an army](slices/20-ai-inherits-an-army.md)
- [ ] [21 Frontline rules](slices/21-frontline-rules.md)
- [ ] [22 Army hand-over](slices/22-army-hand-over.md)
- [ ] [23 Seat reset](slices/23-seat-reset.md)
- [ ] [24 Frontline placement](slices/24-frontline-placement.md)
- [ ] [25 Frontline service](slices/25-frontline-service.md)
- [ ] [26 Briefing card and Play](slices/26-briefing-card.md)
- [ ] [27 Closeout](slices/27-closeout.md)

## Contracts this changes

| Contract | Today | After |
|---|---|---|
| **Order** (page to battle) | The page stamps side, player and a sequence number it counts itself (`web/src/battle/sim/client.ts:359`) | The page sends an order and a local id. The battle stamps side, player and sequence (slice 03). |
| **Publication** (battle to page) | One publisher, one side at a time (`crates/game-wasm/src/lib.rs:650`) | One publisher per side; the authority publishes a list of sides (slice 02). |
| **Seat** | A position, `blue-1`, renumbered when a neighbour is removed (`apps/battle-lab/src/skirmishSetup.ts:36`) | An id that never moves, holding a human, an AI, or nothing (slice 04). |
| **Wire** | None | [wire.md](wire.md): a record stream down, one request per batch of orders up (slices 05, 07). |
| **Room** | None | A room is lobby state on the server; the page draws a view of it and sends actions (slices 14, 15). |
| **Match rules** | One rule set (`fixtures/skirmish.json`) | Frontline overrides two values: the points target and the instant win (slice 21). |
| **Unit ownership** | A unit never changes owner | Frontline only: two recorded seat events, the hand-over and the reset (slices 22, 23). |
| **Build identity** | Shown on screen only (`apps/battle-lab/src/buildIdentity.ts`) | The server refuses a page whose commit or simulation file differs from its own (slice 09). |

The page's battle code keeps speaking the existing page-to-authority protocol
(`web/src/battle/sim/protocol.ts`), and the server hosts the existing authority
(`web/src/battle/sim/authority.ts`). Everything new sits between those two.

## The ladder

`→` marks what a slice needs first. Slices with no arrow can start at once.

| # | Slice | Needs | Proof you can run or see |
|---|---|---|---|
| | **Foundation** | | |
| 01 | Host and endurance probes | | A table: late ticks for a 5v5 on the host; tick time, memory and snapshot size over two hours |
| 02 | Per-side publisher | | The authority lab still switches sides; two sides publish as deltas |
| 03 | The sim stamps orders | | An order after an AI held the seat is applied |
| 04 | Seats and setup the server can run | | Skirmish plays as before; removing a seat renumbers nothing |
| 05 | Record stream and seat link | | A stream cut at any record resumes with nothing lost or repeated |
| 06 | Battle host | → 02 03 04 05 | A 5v5 of AIs on a worker thread; two seats order through a loopback link |
| 07 | HTTP transport | → 05 06 | `bun run server`; a cut stream resumes over real HTTP |
| 08 | **Command any seat** | → 07 | **Two browsers, one side each, against the local server** |
| 09 | Identity, build check, deploy | → 08 | The same battle through the deployed server |
| 10 | Playout | → 08 | Bursty arrival plays as even motion |
| 11 | Yours is blue | → 08 | A Red commander's screen reads as a Blue commander's does |
| 12 | Your side at the bottom | → 08 | A Red commander's own edge is at the bottom of the screen |
| 13 | Native and WebAssembly parity | | The first tick where the two builds disagree, and why |
| | **Rooms** | | |
| 14 | Room model | → 04 05 | Every room rule as a test with no network |
| 15 | Room service | → 07 09 14 | Two terminals watching one room change |
| 16 | Multiplayer screen | → 15 | Menu tile, room list, create, join by code |
| 17 | Room lobby screen | → 15 | Host and guest in one room, seats moving live |
| 18 | Room battle | → 08 15 17 | Start, drop, return, finish, result |
| 19 | **Milestone** | → 09 to 12, 16 to 18 | **Two people on two networks finish a room battle** |
| | **Frontline** | | |
| 20 | An AI inherits an army | | An AI handed 30 and 150 fighting units keeps its objectives |
| 21 | Frontline rules | → 01 13 | Holding every objective no longer ends a Frontline match |
| 22 | Army hand-over | → 20 | A recorded event moves a seat's army; the replay matches |
| 23 | Seat reset | → 22 | A recorded event empties a seat; the replay matches |
| 24 | Frontline placement | → 05 | Every placement rule as a test with no network |
| 25 | Frontline service | → 18 21 to 24 | Join, leave, idle out, return, last one out |
| 26 | Briefing card and Play | → 25 | One click, a briefing, Deploy, fighting |
| 27 | Closeout | → all | The one full run; docs; the plan archived |

The same graph, drawn: [roadmap.html](visualizations/roadmap.html).

## One owner per concept

Every pass inherits these. The end state should read as designed today, with
no part of it bolted on beside an older twin.

| Concept | Owner |
|---|---|
| Ticking, accepting orders, publishing | `web/src/battle/sim/authority.ts`, in the browser and on the server. There is no second tick loop. |
| Order side, player and sequence stamp | The simulation, through the WebAssembly handle (slice 03) |
| Publication deltas and baselines | `crates/sim/src/publication.rs`; one publisher per side in the handle |
| Page-to-authority messages | `web/src/battle/sim/protocol.ts` |
| Wire types, record framing, ring, resume | `packages/multiplayer/src/wire.ts` and `stream/`; battle, lobby and list streams all use them |
| Transport | `apps/game-server/src/http/` and `web/src/battle/remote/httpLink.ts`. A WebSocket later replaces these two and nothing else. |
| Battle host (worker thread, per-side fan-out) | `apps/game-server/src/battleHost/` |
| Presence (who is connected, and for how long they have been gone) | `apps/game-server/src/presence.ts`. Rooms and Frontline read it; nothing else times a disconnect. |
| Seat identity and the lobby's rules | `packages/skirmish-setup/src/`; offline skirmish and rooms share it |
| Building a battle request from a lobby | `packages/skirmish-setup/src/`; the page and the server call the same function |
| Room state and rules | `packages/multiplayer/src/room.ts`. The server holds the rooms; a page holds no copy, it draws a view. |
| Frontline placement policy | `packages/multiplayer/src/frontline.ts` |
| Frontline's rules and seat events | The simulation |
| Build identity | `apps/battle-lab/src/buildIdentity.ts` on the page, `build.json` on the server, compared once in `/hello` |
| Callsign and browser id | `apps/battle-lab/src/online/identity.ts`; the clash suffix is the server's |
| Server address | `apps/battle-lab/src/online/server.json` |
| The host | `apps/game-server/deploy/` |

One piece of scaffolding is planned, and it has an end: the dev battle (an
endpoint and a page that seat two browsers in one fixed battle) arrives in
slice 08 and is deleted in slice 18, when a room can start a battle.

## Gates every slice inherits

- **Tests.** Write the test first, with
  [write-tests](../../.agents/skills/write-tests/SKILL.md). Test moments the
  game rules in or out, never a roster, a stat or a class name. New TypeScript
  tests live in `web/tests/multiplayer/` and run in Vitest's node environment.
- **Digests.** A slice that should not change outcomes leaves every existing
  battle digest and replay unchanged. Slices 21 to 23 change rules on purpose
  and say which digests may move.
- **The check ladder.** Climb only as far as the slice's question needs
  ([development.md](../../development.md#check-ladder)). The one full run is
  slice 27.
- **Anything the player sees.** Use
  [game-ui](../../.agents/skills/game-ui/SKILL.md) before designing it. Judge
  the shot against its target with
  [compare-screenshots](../../.agents/skills/compare-screenshots/SKILL.md).
  Finish with an unprimed
  [screenshot-critique](../../.agents/skills/screenshot-critique/SKILL.md),
  the last check before the slice is accepted. Then show the user with
  [preview-shots](../../.agents/skills/preview-shots/SKILL.md): give them about
  five minutes, and if they are silent, decide on the evidence, record the
  decision in [decisions.md](decisions.md), close the shots and carry on.
- **Mechanics.** Use
  [tweak-mechanics](../../.agents/skills/tweak-mechanics/SKILL.md) before
  slices 21 to 23 propose anything.
- **Renderer and camera work.** Use
  [renderer](../../.agents/skills/renderer/SKILL.md) first (slices 11, 12).
- **Review.** Run [review](../../.agents/skills/review/SKILL.md) before any
  push to `staging`.

## Firewalls

- No compatibility layer and no migration. Each contract change is a hard
  cutover in one slice.
- `packages/multiplayer` and `packages/skirmish-setup` import nothing from
  `apps`, the renderer, the DOM or Node, and use relative imports only. The
  server's whole import graph has no bundler alias and no `?raw` or `?url`
  import.
- The battle host, the room code and the Frontline code never import
  `node:http`. `web/src/battle/remote/` is the only page code that knows a
  server exists.
- Rooms (slices 14 to 19) change nothing under `crates/`.
- Slices 22 and 23 touch no room path. A room battle records no seat event.
- A page can send orders and previews, and nothing else: no pause, speed,
  start, side switch, replay export or controller change.
- The server never logs or echoes a request header: the host's edge forwards
  its own credentials in them.
- No database, account, chat, word filter, faction pick, 10v10 or
  per-player publisher.

## Open items

| Item | Question | Burned in |
|---|---|---|
| O1 | Can the host hold a real 5v5 at 30 ticks a second? | [01](slices/01-probes.md) |
| O2 | Where do native and WebAssembly first disagree? | [13](slices/13-native-wasm-parity.md) |
| O3 | How fast does a battle slow with age? | Answered in [01](slices/01-probes.md): tick cost grows slowly, memory grows about 1.4 GB an hour without levelling off. Read by [15](slices/15-room-service.md) and [21](slices/21-frontline-rules.md). |
| O4 | How big is a join snapshot after hours? | Answered in [01](slices/01-probes.md): 2.6 MB at two hours. Not a problem. |
| O5 | Does an AI cope with an army it didn't buy? | [20](slices/20-ai-inherits-an-army.md) |
| O6 | Is 300 ms or more per order acceptable? | [09](slices/09-identity-build-deploy.md), judged at [19](slices/19-milestone.md) |

A slow host does not stop the build: the user has said twice that server speed
is not a blocker. A miss on O1 or O6 is recorded, and the host swap changes one
folder and one address.

## Deferred, on purpose

- **Forgetting the dead.** Every unit ever bought stays in the per-tick loops
  (`crates/sim/src/battle/dispatch.rs:237`), and a battle's memory grows by
  about 1.4 GB an hour without levelling off (slice 01). An hour holds, so
  Frontline's target starts there. Finding what grows and trimming it is its
  own plan, needed before a game may run much longer.
- **A faster host, WebSockets, native battle processes.** Each is a swap behind
  a seam this plan builds.
- **Answering previews in the browser.** The fallback if server previews feel
  too slow after slice 19.
- **A publisher per player, 10v10, rematch, chat, a callsign word filter.**

## In this folder

- [decisions.md](decisions.md): the givens from the walk, what was rejected,
  and the ledger of what this plan decided where the walk was silent.
- [wire.md](wire.md): the wire contract.
- [research.md](research.md): external practice, with sources.
- [visualizations/unknowns-map.html](visualizations/unknowns-map.html): the
  walk's record, with the measurements and the nine landmines.
- [visualizations/lobby-mock.html](visualizations/lobby-mock.html): the
  approved screen sketch. Its players and rooms are invented.
- [visualizations/roadmap.html](visualizations/roadmap.html): the ladder as a
  picture.
