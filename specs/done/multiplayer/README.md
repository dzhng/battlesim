# Multiplayer: rooms and Frontline

Online play on one set of servers. **Rooms:** a host makes a public or
private room, which is the skirmish lobby with several humans on either
side, and starts a battle. **Frontline:** Play shows a briefing for a seat
in a battle already running, on the side that is losing, or in a new battle
when none has a seat; Deploy takes it. Every seat is played by a human or by
its own AI.

It has been live on the production site since 2026-11-16. A lobby answers
the menu; every battle runs on a machine of its own, rented when the battle
starts and given back when it ends.

This is the record of why it is built the way it is. The code says how:
start from the [server's README](../../../apps/game-server/README.md), the
[contract package](../../../packages/multiplayer/README.md) and
[the wire](../../../packages/multiplayer/wire.md). Every decision is in
[decisions.md](decisions.md), with who made it or what it replaced.

## What the user asked for

Six preferences shaped everything ([decisions.md](decisions.md) lists them
with the decisions that follow):

1. An alpha: the simplest thing that works.
2. Nothing persists. No database, no accounts; a restart wiping everything
   is fine.
3. Use what is at hand and defer scale.
4. One path for everything: rooms and Frontline share one server and one
   rule set.
5. Zero friction: one click to be in a fight.
6. Normal rules, with resources as the only balance lever.

## Why it is built this way

**The server runs the battle, and a page only watches its own side.** The
game's fog is real: each side is sent only what its own units can see, so a
player's machine never holds the enemy's hidden state, and a host leaving
does not end a match. The page builds the map itself from the battle's
settings and refuses to play unless its map is the server's, hash for hash;
it never simulates.

**The server decides who gave an order.** A page says what it wants; the
server knows which seat that browser sits in and stamps the order's side,
player and place in sequence itself. What a page claims about itself is
never read. That is the whole of the trust model, and why the simulation
stamps orders (`Battle::accept_order`).

**A stream down and small requests up, not a socket.** The first host
refused WebSocket upgrades, and a measured stream carried the same data
unbuffered. One record framing serves every stream. A battle's reader
resumes by record number after a cut and takes a fresh snapshot only after a
real gap; a room or the list is small enough to be sent whole on every open.
WebSockets were measured again on the rented machines, which allow them:
four milliseconds faster out of about 175. Not worth a second transport.

**One publisher for each side, and the server never waits for a player.**
A reader who falls behind, or returns from a hidden tab, is sent a fresh
snapshot; the battle's clock is the server's.

**The lobby is small and stays put; every battle gets its own machine.**
The first host, a VM, ran a full ten-AI battle at about half speed from its
sixth minute. Rented machines ran the same ten minutes about as fast as a
developer's Mac, start in about ten seconds and cost a few cents an hour,
and a machine that exists only while its battle does costs nothing when
nobody plays. So
the lobby (`/hello`, rooms, Frontline, presence) never steps a battle. It
asks a provider for a machine and holds a battle only through a control
surface guarded by a key it made for that battle. Locally the provider is a
child process, which is how the same split is tested on one computer.

**A page may play when it agrees with the server on what they share.** The
first rule was "same commit". Under it every promotion of the site, even
one that touched nothing the two ends share, refused every player until the
lobby was redeployed. The rule is three hashes, made from the files
themselves: the simulation's bytes, the documents a battle is built from,
and the wire's sources. Promotion asks the live lobby whether it admits the
promoted site and redeploys it from the promoted commit when it does not,
or cannot be reached.

**Frontline needs two rules the game did not have, and they are seat
events, not special cases.** A joiner starts with no units, and with the
opening credits the first time they join that game. The army they displace
must go somewhere: it passes to the AI seat on that side with the fewest
units, and the seat is then reset.
Both are recorded in the replay at the tick they apply, and both happen at
one tick so no page ever sees a seat half changed. Everything else about
Frontline is the skirmish rules with a longer game: a higher points target
and no instant win for holding every objective.

**In battle your side is blue and at the bottom, whichever side you
command.** The lobby does not turn: its sides stay left and right, so
moving to a seat across the room does not flip everything.

**Security is one limit and the plain bugs fixed.** An independent read
before launch listed the ways a stranger could misuse the servers. The
user ruled that an alpha gets one simple limit, not a defence for each:
a device may have one game of its own making at a time, a room's code is
six characters and is not a secret, and nothing throttles guesses.

## What must stay true

- **A browser id and a battle's key are credentials.** Neither appears in
  a URL, a reply to anyone else, a room or list view, a ticket, a briefing,
  history state, an error or a log line. Other players see a callsign.
- **A player acts only for the seat the lobby seated them in,** and is sent
  only their own side's records.
- **Only `apps/game-server/deploy/` and the page's `server.json` name the
  machines' host.** Everything else speaks to a provider of machines and an
  address. No check enforces this; a search of the servers, the shared
  packages and the page for the host's name does.
- **No simulation change rides in on multiplayer.** Seat events are outside
  the digest, and a battle with none records none and replays to itself
  (`seat_replay`).
- **Rooms and Frontline share one of everything:** one count of places,
  one presence module, one record stream, one way of hosting a battle, one
  lobby view with two drivers (offline and a room).
- **A lobby deploy ends the battles then running.** Accepted for the alpha.
  Promotion deploys only when what the two ends share has changed or the
  lobby cannot be reached; a deploy pushed by hand always does.
- **A player's unreadable input is refused to that player and the battle
  goes on.** No request a player can send ends a battle. A battle ends when
  it is won, when the simulation traps, when its lobby ends it (a room
  nobody is connected to, a Frontline game with no human left), when its
  lobby goes silent, or when the lobby sends it something it cannot apply.

## Where the code is

| Concept | Owner |
|---|---|
| The wire's types, the record stream and its resume rule, the room model, Frontline placement | `packages/multiplayer/src/` |
| Frontline's match rules: the longer game | `fixtures/frontline.json`, `MatchRuleSet::Frontline` in `crates/sim/src/skirmish.rs` |
| Seats, sides and the lobby as a battle request | `packages/skirmish-setup/src/` |
| The lobby: rooms, Frontline, presence, places, hosting | `apps/game-server/src/` (`rooms.ts`, `frontline.ts`, `presence.ts`, `online.ts`, `hosting.ts`) |
| The battle server and its control surface | `apps/game-server/src/battle.ts`, `http/control.ts` |
| A battle on its thread; candidates prepared side by side | `apps/game-server/src/battleHost/` |
| Machines: a child process, or a rented one | `apps/game-server/src/machines/`, `apps/game-server/deploy/sandboxes.ts` |
| Deploying the lobby; promotion keeping it in step | `apps/game-server/deploy/`, `tools/promote-main.mjs` |
| What makes two builds the same | `packages/multiplayer/buildIdentity.ts` (what is hashed), `sameBuild` in `packages/multiplayer/src/build.ts` (the comparison) |
| The page's link to a battle, and playout | `web/src/battle/remote/` |
| The Multiplayer page, rooms, the briefing card | `apps/battle-lab/src/online/` |
| Seat events and the referee's view | `crates/sim/src/battle/seats.rs` |
| Publishing each side from its own baseline | `crates/sim/src/publication/publisher.rs` |

The tests are in `web/tests/multiplayer/`. Those of the servers start real
local ones and speak to them over HTTP; the rest test the shared rules, the
page's screens and the deploy tools directly. The simulation's are `seat_hand_over`, `seat_reset`, `seat_replay`
and `frontline_rules` under `crates/sim/tests/`. The scenes `multiplayer`,
`room-lobby`, `online-battle` and `frontline` play both modes in a browser.

## What was measured

The numbers behind the choices are kept in
[assets/host-comparison-2026-11-15.txt](assets/host-comparison-2026-11-15.txt),
sections A to N. The ones that decided something:

- **Hosts:** in a ten-minute soak of ten AIs the VM had 47% of ticks over
  budget and shed 30% of them. The smallest rented machine had 18% and 4%,
  and the size battles rent 19% and 6%, against a developer's Mac at 18%
  and 5%. That chose a machine for each battle.
- **A rented machine is not always a fast one.** A later soak, run to its
  end at 42 minutes, kept 47 to 67% of real time from its fifth minute to
  its thirtieth. The machines are shared and were seen with three CPU
  models.
- **Bandwidth is not the constraint.** A seat's stream is about 1 Mbit/s in
  a full battle; a rented machine's route carried 38 Mbit/s without
  queuing.
- **An order's round trip** is about 180 ms from California to a battle's
  machine in France. Distance, not the transport.
- **Play on an empty server** took 16 to 52 seconds over six tries. One
  machine prepares a new game's candidate maps side by side; trying them one
  after another, the worst was 77.
- **Memory** grew about 65 MB a minute in that long soak and reached 3 GB
  at 42 minutes, which is why a battle's machine has 8 GB.
- **Native and WebAssembly builds play the identical battle** where they
  were given one scenario and seed: every minute's digest for fifteen
  minutes of the server's battle, eighteen maps byte for byte, and the
  endurance battle at 31 checkpoints over fifteen minutes
  ([parity-endurance-checkpoints.txt](assets/parity-endurance-checkpoints.txt)).

## Dead ends

- **A hosted database service as the server, or the host's browser running
  the room.** The first cannot run a battle; the second holds the whole
  battle on one player's machine and dies with them.
- **The battle on the lobby's own machine.** Half speed with ten AIs.
- **"Same commit" as build identity.** See above.
- **One machine for each candidate map.** A refused map then cost a whole
  rental; one machine prepares them all.
- **A native simulation in the battle server.** Built at the `x86-64-v4`
  level it is a fifth faster on the ten-AI battle, and its median tick is still
  over budget from the sixth minute to the eleventh. Not built: a second
  binding and a build at every deploy, for a fifth.
- **A throttle on room-code guesses, and a room code of a word and four
  digits.** Both built, both removed by the user's ruling on security.
- **Pinned digest streams to prove seat events changed nothing.** They
  matched staging's at every merge, and moved with every change to movement
  by anyone. Retired for a check that a battle without seat events records
  none and replays to itself.
- **A joiner inheriting the seat as it stands; a catch-up bonus; one AI
  playing every empty seat; picking a faction in Frontline; the lobby
  turning to face you.** Each considered and turned down;
  [decisions.md](decisions.md) says why.

## Left open

- **A full battle of ten AIs runs slower than real time on every machine
  measured,** a developer's Mac included, from about the fifth minute. No
  host fixes it and a native build does not either. It is the simulation's
  step cost with large armies, and helping it would help every player's
  browser too.
- **Two people on two networks have not played a room battle.** Two browser
  pages on the production site have, from one computer: both in the battle
  and at thirty ticks a second (section O of the measurements). How it
  feels with someone far away has not been tried.
- **A battle's machines were all in France.** A player in Europe should be
  close; nobody has measured from there.
- **The provider's limits** on machines at once and starts a minute are
  not probed, and what a played battle costs has not been read back; only
  the comparison's machines were. The lobby's limit of eight battles is a
  first guess.
- **Nothing tells a player that an opponent dropped and an AI took over,**
  or came back, and the result screen does not say why a battle ended.
- **An AI handed a very large army takes several rounds to order all of
  it.** It gives its orders in one burst and the per-player allowance lets
  thirty through a second; a seat holding two armies is fully ordered in
  about ten seconds. The user left the pace as it is for the alpha.
- **The shareable skirmish link keeps a position-based copy of the seat
  model,** because a link distinguishes a passive AI from an active one and
  a lobby seat does not.
- **A trap inside a building preview is swallowed** where the other two
  previews let it end the battle. It predates this work.

## What it was drawn against

The lobby's four screens were sketched before any were built, in
[visualizations/lobby-mock.html](visualizations/lobby-mock.html): a
click-through with invented players and rooms, made for the user to choose
between options. Captures of the four screens as chosen are
[screen 1](assets/lobby-mock-screen1.png) (the menu's tile),
[screen 2](assets/lobby-mock-screen2.png) (the Multiplayer page with the
Frontline band), [screen 3](assets/lobby-mock-screen3.png) (the room) and
[screen 4](assets/lobby-mock-screen4.png) (the briefing card). They drove
structure and reading order, not looks: the built screens use the game's
own menu panels, and the mock's 5,000-point target and "arrive by road 2"
callout were placeholders: the built card shows the battle's real target
and promises no road.

[visualizations/unknowns-map.html](visualizations/unknowns-map.html) is the
map of what was unknown before the plan was drawn up, with the hosting
probes' first results. Its code citations are of that day.

The Multiplayer tile's picture was generated; its model and prompt are in
`assets/runtime/menu/provenance.json`. The user asked for people in front
with vehicles and helicopters behind, and for the heads not to be cropped;
the tile's styles hold the crop.
