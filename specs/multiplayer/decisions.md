# Multiplayer decisions

Four parts. Part 1 is settled by the user or by measurement and is not
reopened. Part 2 is what lost, and why. Part 3 is what this plan decided where
the walk was silent; the user may overturn any row. Part 4 is where an
implementer records a decision a slice forced on them.

Numbers in part 1 are the walk's own, so they match
[the map](visualizations/unknowns-map.html), which also holds the evidence.
"You" is the user. "Agent" is an answer given on the user's behalf under the
delegation below and shown to them at the time. "Territory" is a fact the code
or a measurement settled.

## Preferences the user confirmed

1. Alpha first: the simplest thing that works. When two answers fit, the
   simpler one wins.
2. Nothing persists: no database, no accounts. A restart wiping everything is
   fine.
3. Use what is at hand and defer scale: Bento and Node now, faster servers
   later.
4. One path for everything: rooms and Frontline share one server and one rule
   set.
5. Zero friction for the player: one click to be in a fight.
6. Normal rules, with resources as the only balance lever.

The user delegated plumbing questions that follow from these, on 2026-11-14.
Game design stays with the user.

## Part 1: the givens

### Server and transport

| # | Decision | By |
|---|---|---|
| 1 | One game server runs every online battle, rooms and Frontline alike. Hidden enemies stay off players' machines, and a host leaving does not end a match. | You |
| 4 | No Convex and no database. One Node server holds rooms, names and battles in memory. | You |
| 8 | Bento (workspace `armata`) is a temporary host. Nothing in the design depends on it beyond its address and its deploy step. Server speed is not a blocker. | You |
| 9 | One binary stream down and one small request per batch of orders up. No WebSockets: Bento apps refuse the upgrade. | Territory |
| 10 | A stream resumes by record number after a cut. A full snapshot is sent only after a real gap. Bento cuts every stream at 300 seconds. | Territory |
| 11 | The game page stays on its current host and calls the server cross-origin. | Territory |
| 67 | Bento's public route carries the stream as well as the private one. Each order or preview through it takes about 300 ms from California. | Territory |
| 13 | A battle is the WebAssembly sim on its own worker thread inside the Node server. Native was no faster on Bento, so it stays a later swap. | Agent |
| 57 | One publisher per side. The server never waits for a player. One who falls behind or returns from a hidden tab gets a fresh snapshot, and so do their teammates. | Agent |
| 56 | The server accepts only orders and previews from a page. Pause, tab-hidden, start, side switch, replay export and controller changes are the server's. | Agent |
| 22, 28 | The server stamps each order's side, player and sequence number from the seat. What the page claims is ignored. | Agent |
| 60 | Orders go one batch at a time per player and retry on a timeout. | Agent |
| 55 | Move and purchase previews stay on the server, capped at about five a second per player. The trailing ghost is an accepted rough edge. | Agent |
| 20 | No pause or speed control online. | Agent |
| 14 | A player's browser generates the map from the battle's settings and checks its identity against the server's. | Agent |
| 59 | The server is deployed from the same commit as the live site, with the same simulation file and game data. Any other site build is told online play isn't available on it. | Agent |
| 62 | At most one fewer battle than the machine has cores. Beyond that the menu says the servers are full. | Agent |
| 23 | The room list and the lobby are streams that resend the whole state when it changes. | Agent |
| 58 | The shared lobby model is split free of bundler-only imports first, and the server joins the existing checks. | Agent |

### Identity

| # | Decision | By |
|---|---|---|
| 5 | A callsign is remembered by the browser and shown as `name#2` on a clash. | You |
| 46 | The player is never asked: a random callsign is filled in and can be edited in the header. | You |
| 17 | The browser makes a random id once and keeps it beside the callsign. That is how the server knows a returning player. | Agent |
| 63 | A callsign has a length and character limit only. No word filter in alpha. | Agent |

### Rooms

| # | Decision | By |
|---|---|---|
| 33 | The host runs the room: map, size, factions, seats and start. Start needs every human marked ready. If the host leaves, the human who has been there longest becomes host. | You |
| | Players move freely to any open seat on either side. The host sets a seat open or AI, and deletes seats. There is no closed seat. | You |
| 39 | A seat still open when the battle starts is dropped. Only a seat set to AI is played by AI. | You |
| 35 | Nobody joins a room battle after it starts. A player who drops has their seat run by its AI, with the army it has. | You |
| 37 | A dropped player can return to their own seat at any point while the battle runs. Nobody else can take it. | Agent |
| 19, 49 | A private room's code is made by the server, looks like `TANGO-4821`, and is also the room's id and its link. | You, Agent |
| 21, 38 | A room is deleted when no human has been connected for about a minute. | You, Agent |
| 41 | Open seats don't count toward the map's size or roads. Only seats holding a human or an AI do. | Agent |
| 42 | Each side needs at least one human or AI before the host can start. | Territory |
| 43 | Seats close up at the start, so roads follow the filled seats in order. | Agent |
| 48 | Rooms whose battle has started are not listed. | You |
| 50 | No chat in the lobby in alpha. | You |
| 34 | A room battle needs no new battle rules. The fresh start and the army hand-over are Frontline's alone. | You |

### Frontline

| # | Decision | By |
|---|---|---|
| 2 | Frontline is the normal rules with a higher points target. Its length is not a requirement. | You |
| 7 | Frontline turns off the instant win for holding every objective; holding everything just scores fastest. Rooms keep it. | Agent |
| 54 | The points target starts at about an hour's play and rises once battles stop slowing with age. | Agent |
| 21 | A Frontline game is created the moment a human needs one and deleted when its last human leaves. | You |
| 24 | No faction pick. One button, and you play what your side fields. | You |
| 25 | A new game gets two different factions drawn at random. | Agent |
| 26 | The button picks a running game whose losing side has a free seat, fullest first; if none, a new game. A game past 90% of its target is closed to joiners. The losing side is the one with the lower score. | You, Agent |
| Q5 | Five seats a side, each played by a human or by its own AI. Every seat earns the same whoever holds it, so both sides always total the same. There is no other balance lever. | You |
| 30 | A joiner gets the lowest-numbered seat an AI holds, from seats 1 to 4. Seat 5 is always AI. | Agent |
| 6 | A joiner starts fresh with the opening credits and no units. | You |
| 29, 31 | The army and credits a joiner displaces, with purchases still on their way, pass to the AI seat on that side with the fewest units. | Agent |
| 6 | When a human leaves, their seat's AI carries on with their units and credits, and the seat is open to the next human. | You |
| | Starting credits are granted once per player per game. Leaving and rejoining does not collect them again. | You |
| 18 | If a connection drops, the AI drives the seat at once; back within about a minute and it is yours again. | Agent |
| 61 | After about three minutes without an order, an idle human's seat returns to its AI and reopens. | Agent |
| 27 | When a game ends the result shows, everyone returns to the menu, and the game is deleted. | Agent |

### What the player sees

| # | Decision | By |
|---|---|---|
| 44 | One Multiplayer tile on the main menu, above Skirmish. Frontline and the rooms are inside it. | You |
| 45 | The drop-in mode is called Frontline. | You |
| 47 | A room row says the host, the map and the seats. | You |
| 51 | Dropping in shows a briefing card first (side, seat, credits, score, who is fighting), then Deploy. It also covers the 15 seconds a browser needs to build a large map. | You |
| 64 | In battle your side is always the accent blue and the enemy is always red, whichever side you command. The lobby still names the sides Blue and Red. | You |
| 65 | In battle, your side is always at the bottom of the screen. | You |
| 66 | The lobby does not turn. Its sides stay left and right, and its map picture keeps one orientation for everyone: moving to a seat on the other side must not flip everything. | You |
| | Ready shows only when true. Seat strips in the room list: solid for a player, dashed for open, dim for AI. "N VS M" counts only seats that will fight. | You |
| | The room reuses the skirmish lobby's layout. A guest sees the settings but can't change them. | You |

### Order of work

| # | Decision | By |
|---|---|---|
| 40 | Foundation, then rooms, then Frontline. | You |
| 53 | The first milestone is a friend and the user finishing a room battle over the internet. | You |
| | The alpha is played by friends sent a link and by anyone who finds the site, worldwide. | You |
| | No backward compatibility and no migrations: hard cutovers. | Default, not opted out of |

## Part 2: rejected on the way

| Rejected | Why |
|---|---|
| Convex for rooms and placement | It cannot run a battle (one second per function, thirty minutes per action, no hand-off of a running battle), and the user wanted no database at all. |
| Rooms run in the host's browser | The host's machine would hold the whole battle, the match would die with the host, and it is a second path. |
| A native Rust process per battle | About twice as fast on the Mac, level with Node on Bento, and it needs a Linux build per host. It stays a later swap behind the battle host. |
| WebSockets | Bento refuses them. The probe showed a stream and requests carry the same data without buffering. The transport stays replaceable. |
| A joiner inherits the seat as it stands | Needs no new mechanic, and was recommended twice. The user chose a fresh start after seeing the cost. |
| A catch-up bonus for the trailing side | It punishes a side for playing well and adds a number to tune. Joiners going to the losing side is the only catch-up. |
| One AI playing every empty seat as a single player | Every seat already has its own AI, and one player is limited to 30 orders a second. |
| Picking a faction in Frontline | A side fields one faction, so picking a faction would pick the winning side as often as not. |
| A host who can start without a ready check | The user chose the ready check. |
| An open room seat played by AI; joining a room battle after it starts | Both overturned by the user: open is not AI, and drop-in rules stay in the drop-in mode. |
| The lobby turning so your side is at the bottom | Overturned by the user: battle only. |
| Splitting fog across cores | A direction, never confirmed against the fog code. Not in this plan. |
| 10v10 | The rules allow ten seats a side; maps and speed do not yet. Later. |

## Part 3: decided by this plan

Each row is a call made while slicing, with the alternative that lost. Rows
marked ★ bend or extend a given, or touch game design: the user should look at
those first.

| # | Decision | Rejected alternative |
|---|---|---|
| P1 | Shared code lives in two new packages, `packages/multiplayer` (wire, stream, room, placement) and `packages/skirmish-setup` (seats, lobby model, request builder). The server is `apps/game-server`. Page transport is `web/src/battle/remote/`. | Everything under `web/src/online`; a top-level `server/` with its own manifest. `web/tsconfig.json` and the linters already cover `packages/*/src` and `apps/*/src`. |
| P2 | The server runs under Node as plain JavaScript bundled with `bun build`, locally and deployed. Its import graph uses relative imports only and reads the simulation file and game data from disk, as `web/src/maps/node.ts` already does. | A second Vite config carrying the site's aliases and `?raw` imports into the server. |
| P3 | The simulation stamps an order's sequence, for local play as well. | A counter kept by the server: an AI's own orders advance the seat's counter (`crates/sim/src/battle.rs:1117`), so a mirror drifts, which is today's refused first order after a takeover. |
| P4 | Battle, lobby and list streams share one record framing and one reader. A state stream resends the whole state on open and on change. | Server-sent events: `EventSource` cannot send the `X-Player` header, would be a second reconnect path, and is cut sooner (240 seconds). |
| P5 | The browser id travels in an `X-Player` header and is the only credential. | Cookies (cross-origin, and ambient credentials invite forged requests); the `Authorization` header (the host's edge puts its own credentials in headers). |
| P6 | An order's acknowledgement returns in its request's reply. Previews use their own request: newest wins, never retried. | Acknowledgements on the shared side stream; previews queued with orders, where a slow preview would hold an order back. |
| P7 ★ | A player counts as gone after about five seconds with no stream open, not the instant one closes. One grace, kept by one presence module, serves both cases: in a battle the seat's AI takes over then (refining decision 18's "at once"), and in a lobby the player has left then. | Acting on every stream close: the host cuts every stream each 300 seconds, so the AI would seize and release every army, and every lobby would empty, every five minutes. A separate timer for each case. |
| P8 | The server starts a battle ticking as soon as it is made. The existing preparation phase (60 seconds, shorter when everyone is ready) covers map building in the browsers. | A new gate that waits for every player's stream. |
| P9 | Build identity is the commit plus the SHA-256 of the simulation file. The site build publishes that file and a `build.json`; the deploy step downloads them from the promoted site and verifies them. The server never builds its own simulation. | Pinning the WebAssembly toolchain and rebuilding; publishing the whole server bundle from the site. |
| P10 | The server's address is committed data. A page on localhost uses the local server. A dev build accepts `?server=`. | A build-time variable. |
| P11 ★ | A room holds one of the server's battle slots from the moment it is created, so "servers are full" is said at creation and at Frontline's Play. | Refusing at Start, after people have gathered and readied. |
| P12 | Joining a room seats you in the first open seat, on the side with fewer humans. A room with no open seat refuses you. Nobody is in a room without a seat. | Unseated members watching the lobby. |
| P13 | A player who has left a lobby, by the Leave control or by being gone (P7), frees their seat, and the host passes on if it was theirs. | Holding a lobby seat for an absent player. |
| P14 ★ | After a room battle the result shows, everyone returns to the menu and the room is deleted, as Frontline does (decision 27). | Returning to the lobby for a rematch. |
| P15 | A ticket carries the battle's settings. Both ends build the battle request with one shared function, and the page refuses to start if its map identity differs from the server's. | Sending the prepared request, which repeats game data the page already has. |
| P16 | The simulation picks the seat that receives a displaced army, by the rule in decision 29, breaking ties by seat order. The recorded event names only the seat being vacated. | The server picking and naming the receiver: a game rule would then live outside the simulation. |
| P17 ★ | The server remembers which players have had their opening credits in a game and says so in the reset event. A player who rejoins after their seat reopened starts with no units and no credits, and earns income from then. | The simulation tracking humans, whom it does not know. |
| P18 ★ | A new Frontline game draws its map kind and region at random and uses the smallest size that seats five a side. | A fixed map. |
| P19 ★ | When scores are level, the losing side is the one with fewer humans, then a coin toss. | Always Blue. |
| P20 | Frontline's Play reserves the seat for about a minute while the briefing shows. The hand-over and reset happen at Deploy. | Taking the seat at Play, which idles an army while the map builds. |
| P21 | The battle handle gains a small referee view for the server: scores, the target, and each seat's controller and unit count. | The server decoding a side's publication to read the score. |
| P22 | The server closes a socket whose queue passes a bound. A page more than about two seconds behind reopens its stream without `after` and takes a snapshot; under that it fast-forwards. | The server slowing the battle for a reader; a page playing a long backlog at speed. |
| P23 | A dev battle (one fixed battle that seats two browsers) is scaffolding from slice 08, deleted in slice 18. | Keeping a second way into a battle beside rooms. |
| P24 | Frontline's rules are two overrides on the skirmish rules, not a second rule set. | A full copy of `fixtures/skirmish.json`, which would drift. |
| P25 | Guesses at private room codes are throttled per client address, read from the header the host's trusted proxy sets. The header's name lives with the deploy files. | Throttling per browser id, which an attacker rotates freely. |
| P26 | The battle cap is the smaller of cores minus one and what slice 01's memory figure allows. Refines decision 62. | Cores alone: one battle used 556 MiB after ten minutes. |
| P27 | A miss on O1 or O6 is recorded and the build continues. | Stopping to find a host first. |
| P28 | The offline skirmish lobby uses the same seat model as rooms, and its one human may sit on Red. | Two lobby models. |

## Part 4: decided while building

Append here: the slice, the decision, why, and what it replaced.
