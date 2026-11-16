# Multiplayer: rooms and Frontline

**Status:** live. Online play was promoted to production on 2026-11-16 (commit `2c2328133`): the site points at the lobby, and each battle runs on its own sandbox. What is left is the milestone with a second person, the native simulation, and the closeout's sweep. **Updated:** 2026-11-16.

Two online modes on one game server. **Rooms:** a host creates a public or
private room, which is the skirmish lobby with several humans on either side,
and starts a battle. **Frontline:** one button drops a player into a battle
already running, on the side that is losing; every seat is played by a human or
by its own AI.

## Next Agent Prompt

You are building online multiplayer from this plan with
[implement-spec-with-codex](../../.agents/skills/implement-spec-with-codex/SKILL.md):
Codex writes each pass and you integrate it. Read
[decisions.md](decisions.md) first (parts 1 to 3 are settled; part 4 is what
the build has decided so far) and [wire.md](wire.md), the one wire contract.

**Watch the disk.** On 2026-11-14 the machine ran out of space during a
simulation test build here, with other sessions building too. Check
`df -h .` before any Rust build or new checkout. Under about 30 GB free, run
Rust passes one or two at a time; under 5 GB, start nothing and ask the user.

**Done and committed:** slices 02 to 08, 10 to 18, 20 to 26 and 28, and the
local half of 09; the endurance half of slice 01. The branch holds staging
as of 2026-11-16 (merge `450c9740d`). **Rooms work end to end on a local server:** two browsers
create a room, start its battle, survive a drop and finish
([how to run it](slices/18-room-battle.md)).

**Pick up here, in this order.**

1. **Slice 19,** the milestone: two people on two networks finish a room
   battle on the production site. It needs the user and a second person.
   Checked without them on 2026-11-16: two browser pages on the production
   site made a room, joined it by its code, started, and both played at 30
   ticks a second.
2. **Slice 27's remaining steps:** the sweep (with Codex since
   2026-11-16; its list is in the slice) and the plan archived.

**Measured and not built: a native simulation in the battle server.** The
user asked for it on 2026-11-16. On the ten-AI battle the server runs, a
native build at the machines' CPU level is a fifth faster than WebAssembly
and plays the identical battle, but still cannot hold thirty ticks a
second ([slice 01](slices/01-probes.md)). The lever is the simulation's
step cost with ten AIs, which is its own work, for another plan.

**Promotion keeps the lobby in step.** `bun run promote:main --yes` asks
the live lobby whether it admits the promoted site and deploys it from the
promoted commit when it does not; on 2026-11-16 it did exactly that. A
lobby deploy ends the battles then running.

**The lobby is live** as the public app `battle-lobby` in the `armata`
workspace, each battle on its own sandbox. What slice 29 still owes is
listed at the end of [the slice](slices/29-sandbox-per-battle.md). **To
deploy it by hand:** build the site (`bun run build:web`), serve `web/dist`
on this computer, and run `bun run deploy:server --site <that address>
--push`. Promotion does this by itself when a promoted site and the lobby
no longer agree.

**Not this plan's, and being handled by another session of the user's:**
since the staging merge of 2026-11-15, four approved `unit-roster` pictures
differ (staging's road and entry changes), and a vehicle ordered through a
town on the default mixed map stopped with "route blocked". Leave both
alone. The `online-battle` scene no longer drives through a town.

**Decided by the user on 2026-11-15: a sandbox for each battle** (decision
68). The rooms and Frontline service stays on the temporary host's VM and
starts a sandbox for every battle. The measurements behind it are in
[slice 01](slices/01-probes.md); slices 28 and 29 build it.

**Looked at on 2026-11-15 at the user's suggestion, and left for later:**

- **WebSockets** work through a sandbox's address and save a few
  milliseconds of about 175 an order. The transport stays as it is.
- **A native simulation in the battle server** halves late ticks when built
  for the machine's CPU, and gains nothing as a default x86 build. Worth
  its own slice after slice 29, as the server's existing binding compiled
  natively, not as a second server.

Both are measured in [slice 01](slices/01-probes.md).

**Known and not this plan's to fix:** the soak sheds ticks on every machine
measured, the Mac included, once ten AIs have large armies. No host fixes
that; it is the simulation's cost.

**Waiting on the user, not blocking:**

- **Deploying** (slices 09, 01 and 19): rooms can now start a battle, so
  nothing technical is in the way.
- **Lane numbers** (slice 12): kept as they are unless the user says
  otherwise.
- **A dropped opponent is silent** (slice 18): nothing tells a player that
  the other side's commander dropped and an AI took over, or came back.
  Not in the plan; worth a line on screen if the user wants it.
- **Frontline's length** (slice 21): 2,100 points, about an hour for the
  leading side on the largest maps. The user's dial.

**Decided by the user so far:** leave the AI's order pace alone for the
alpha (slice 20); the host sets an empty seat from one picker, Open or AI
(slice 17); a room's buttons are the skirmish Reroll's height (slice 17);
the result screen heads a commander's sides "YOUR SIDE" and "ENEMY"
(slice 11); the Multiplayer tile's art shows people in front (slice 16), and the
user approved that art on 2026-11-16, so the main menu's picture is
re-approved with it.

**How to work with Codex here.**

- Pure TypeScript passes run in this checkout (`isolation: shared`), with
  Codex told not to commit and not to touch other slices' files. That costs
  no disk. Do not commit while one is running: the hook stashes the working
  tree.
- A pass that builds needs its own checkout: about 1 GB before any build
  and 1 to 2 GB after. Run at most two, and delete a child's `target/` and
  `node_modules` once its commits are safe on a branch.
- The commit hook refuses growth in a file already over the size budget.
  Split the file by concept; a pass once trimmed a comment to make room.
- Expect to write the proofs yourself when a slice's tests are hard. On
  slice 06 two rounds came back with a test named for each requirement and
  most of them proving nothing; the tests that landed were written at
  integration. Ask each pass to break the behaviour and watch its test fail.
- Read the changed tests first. Every pass so far came back green over a
  weakened gate: an expectation repinned to the new number, an assertion
  deleted with a comment, a rewrite that dropped nine tests, a new test
  asserting the bug it was meant to fix. Say in each brief that an existing
  assertion is never repinned or deleted, then check anyway.
- Then read the model for what the brief did not say: game data copied into
  code, module-level state, a parameter nothing uses, a value invented
  because the function could not know it.
- Record what a pass decided in part 4 of [decisions.md](decisions.md),
  bring wire.md and the slice file up to what was built, tick the checklist
  and update this section before ending a pass.

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
- [ ] [01 Host and endurance probes](slices/01-probes.md) (the endurance half is done; the host half is not)
- [x] [02 Per-side publisher](slices/02-per-side-publisher.md)
- [x] [03 The sim stamps orders](slices/03-sim-stamps-orders.md)
- [x] [04 Seats and setup the server can run](slices/04-seats-and-setup.md)
- [x] [05 Record stream and seat link](slices/05-record-stream.md)
- [x] [06 Battle host](slices/06-battle-host.md)
- [x] [07 HTTP transport](slices/07-http-transport.md)
- [x] [08 Command any seat](slices/08-command-any-seat.md) (first playable checkpoint)
- [ ] [09 Identity, build check, deploy](slices/09-identity-build-deploy.md) (the local half is in; the push and the public-route measurement wait for the user)
- [x] [10 Playout](slices/10-playout.md) (the delay and bound are untuned: nobody has watched it under a jittery network)
- [x] [11 Yours is blue](slices/11-yours-is-blue.md) (one naming question open for the user)
- [x] [12 Your side at the bottom](slices/12-your-side-at-the-bottom.md) (it was already true; now proven)
- [x] [13 Native and WebAssembly parity](slices/13-native-wasm-parity.md) (the two builds agree; nothing to fix)

Rooms
- [x] [14 Room model](slices/14-room-model.md)
- [x] [15 Room service](slices/15-room-service.md)
- [x] [16 Multiplayer screen](slices/16-multiplayer-screen.md)
- [x] [17 Room lobby screen](slices/17-room-lobby-screen.md)
- [x] [18 Room battle](slices/18-room-battle.md)
- [ ] [19 Milestone: a room battle across the internet](slices/19-milestone.md)

Frontline
- [x] [20 An AI inherits an army](slices/20-ai-inherits-an-army.md) (it copes at 30 and is slow at 150; left so for the alpha by the user)
- [x] [21 Frontline rules](slices/21-frontline-rules.md)
- [x] [22 Army hand-over](slices/22-army-hand-over.md)
- [x] [23 Seat reset](slices/23-seat-reset.md)
- [x] [24 Frontline placement](slices/24-frontline-placement.md)
- [x] [25 Frontline service](slices/25-frontline-service.md)
- [x] [26 Briefing card and Play](slices/26-briefing-card.md)

Hosting

- [x] [28 A battle in its own process](slices/28-battle-process.md)
- [x] [29 A sandbox for each battle](slices/29-sandbox-per-battle.md) (deployed; open measurements listed in the slice)
- [x] [30 One machine tries every draw](slices/30-one-machine-every-draw.md) (its two scenes are still owed)
- [x] [31 A build is what both ends share](slices/31-a-build-is-what-both-ends-share.md)
- [x] [33 Six-character rooms, one game a device](slices/33-the-servers-face-strangers.md)
- [ ] [27 Closeout](slices/27-closeout.md) (the full run is done; the sweep and the archive are left)

## Contracts this changes

| Contract | Today | After |
|---|---|---|
| **Order** (page to battle) | The page stamps side, player and a sequence number it counts itself (`web/src/battle/sim/client.ts:359`) | The page sends an order and a local id. The battle stamps side, player and sequence (slice 03). |
| **Publication** (battle to page) | One publisher, one side at a time (`crates/game-wasm/src/lib.rs:650`) | One publisher per side; the authority publishes a list of sides (slice 02). |
| **Seat** | A position, `blue-1`, renumbered when a neighbour is removed (`apps/battle-lab/src/skirmishSetup.ts:36`) | An id that never moves, holding a human, an AI, or nothing (slice 04). |
| **Wire** | None | [wire.md](wire.md): a record stream down, one request per batch of orders up (slices 05, 07). |
| **Room** | None | A room is lobby state on the server; the page draws a view of it and sends actions (slices 14, 15). |
| **Match rules** | One rule set (`fixtures/skirmish.json`) | Frontline overrides three values: how long a game is, the points that is shown as, and the instant win (slice 21). |
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
| 28 | A battle in its own process | → 26 | A room battle with one more process on the computer for each battle |
| 29 | A sandbox for each battle | → 28 | A room battle through the deployed lobby, each battle on its own sandbox |
| 30 | One machine tries every draw | → 29 | Play on an empty server in well under a minute |
| 31 | A build is what both ends share | → 30 | A page plays on a lobby deployed from another commit when nothing they share differs |
| 33 | Six-character rooms, one game a device | → 31 | A six-character code typed in lower case joins; a second game from one device is refused |
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
