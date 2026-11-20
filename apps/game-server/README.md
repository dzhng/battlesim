# Game server

The lobby owns rooms, Frontline, admission and presence. Each battle runs in
its own process, with the simulation on that process's worker thread. The
[battle server](src/http/control.ts) owns that battle's player routes;
[the lobby's hosting owner](src/hosting.ts) reaches it through authenticated
control requests and one resumable event stream. HTTP owns sockets and
credentials, and seating supplied by the lobby binds each player request to the
seat it may command. Neither a ticket nor the battle setup contains a browser
credential.

For local play, from the repository root:

```sh
bun run server
bun run --cwd web dev --host 127.0.0.1 --port 5173
```

Open `http://127.0.0.1:5173/multiplayer` in two independent browser profiles.
Create a room, share its link, ready both players and Start. Tabs in one profile
share the credential kept in local storage. `PORT` chooses the server's listening
port; a development page can use `?server=http://127.0.0.1:<port>`.
The page checks its simulation, battle-data and multiplayer-wire hashes at
`/hello` before opening a room. Different commits play together when those
three agree. The public `GET /build` states only those hashes, without a
credential or player registration.

The local build snapshots its identity and runtime data beside the bundle, so
changing the working directory or rebuilding the checkout cannot change a
running bundle's identity. The [release preparation](deploy/README.md) uses the
site's simulation and an archive of the commit named by the site, regardless
of the checkout’s current commit; it defaults to a dry run.

Rooms reserve battle capacity when created, before anyone gathers or readies.
The [service](src/rooms.ts) holds them in memory; the shared room model alone
owns their rules. Room and list streams use the battle transport's record
framing, with a current snapshot on each open. A room stream opened with a resume cursor must still belong to its member;
a removed player is refused `removed` instead of silently joining again.
Start prepares the room's skirmish on a battle host, then publishes each
member's own ticket with the admitted map identity and battle address. A ticket's callsigns map
contains public names indexed by battle player, never credentials.
The room link restores a returning member's seat while its battle runs.
A finished or failed battle deletes its room and frees the reserved capacity;
existing battle readers receive the final observation before the stream ends.
Preparation failures and crashes close room streams with `battle_failed`.

A dropped member keeps their seat and army, driven by AI until one of the
room's streams reopens for them. A member who chooses to leave a battle is
away from it at once, with no grace, and a battle every human has left that
way ends there and then: nobody is left to play it, and its machine is given
back. Controller changes belong to the battle's recorded history; they change
no lobby membership. Empty-room deletion disposes its host too.

[Presence](src/presence.ts) counts all of a browser's open streams together.
Its disconnect grace protects seats across stream cuts. That count holds a
browser's callsign and Frontline's seats. Each room keeps a count of its own
with the same module, over its state stream and its battle's streams only:
a member reading the room list is connected, and is not in their room. A
room's lifetime is measured from the last human in it, including a room
nobody ever opens.
Callsigns are assigned by `/hello` and held while connected or remembered by an
online mode, including admission still preparing a battle. Once both have
forgotten a browser, its name is available again.

A device may keep one game of its own making alive. [Online state](src/online.ts)
owns that creation claim across rooms and Frontline until removal, including
preparation; joining never takes a claim. [HTTP IO](src/http/io.ts) derives the
device from the configured trusted address header or, on a direct lobby, the
browser id. The client-address trust boundary belongs to
[deployment](deploy/README.md).

[Online state](src/online.ts) owns battle admission and presence across modes.
A lobby holds a slot before it starts; Frontline holds one while preparing or
running. Both hold control operations only; player traffic always goes to the
battle server named by its ticket. Both publish seating through the same hosting owner and use the same battle
transport. The [machine provider](src/machines/index.ts) owns capacity and
allocation; local startup uses child processes, and tests can use real HTTP
listeners inside their process. A machine exposes one address and a stop operation. Player and authenticated
control requests share its listener.

The lobby mints one private key per battle. Only control request headers and
the machine's startup environment carry it. Control replies and events contain
no credentials; stream presence names a battle player and an opaque connection,
which the lobby maps back to its browser. Crash diagnostics remove credentials
before logging. Player orders and previews go directly to the ticket's address;
room and Frontline state stay at the lobby.

A running battle requires a renewable lobby lease. Successful control answers
renew one silence deadline; an individual stalled ping is retried inside it.
[The control protocol](src/http/controlProtocol.ts) owns both ends' timing. A live, bounded preparation
request holds the lease while its map is built. Closing that request stops
preparation; losing the lobby stops a running battle. Locally the child also
observes its parent connection, and lobby shutdown disposes all its machines.

[Frontline](src/frontline.ts) reserves a seat while the briefing builds the map,
then Deploy takes it. A new game draws its candidate maps and initial placements up front; one
machine prepares them concurrently within its CPU and memory budget and picks the
first admitted in draw order. The admitted setup owns the briefing and tickets.
The [battle host](src/battleHost/) retains that world on its preparation thread
and stops the other candidates before readiness.
Its first commander owns their seat from tick
zero; later arrivals displace its army through recorded events. The host sends
both seat events and the human controller in one worker message, and acknowledges
a deploy only after their boundary has been published. A resume changes only
the controller. Placement and lifecycle deadlines belong to the
[pure placement model](../../packages/multiplayer/src/frontline.ts).

A departed commander's army returns to its AI. While another human or reservation
keeps the game alive, its commander can reclaim it during the hold; after the
hold a fresh join receives opening credits only if that person has not received
them in this game. Idle ends only that seat's readers with an `idle` reason.
Disconnects use presence's one grace; Deploy starts that grace again for a page
about to open its first battle stream. The last active human or reservation out
ends the game, even if an AI still holds an absent commander's seat.

[Replay exports](../../packages/multiplayer/wire.md#a-battle-servers-endpoints)
give a seated browser the full command history while its battle server is live.
They include both sides' orders; the ordinary battle stream still carries only
the caller's side's knowledge. Camera history stays with the recording browser.

The referee view is server accounting, read directly from battle state. It is
separate from either side's fog-limited publication. Administrative seat events
and this view are available only to the lobby on the authenticated control
surface; player routes cannot express them.

Frontline's activity is part of the room-list state. The Frontline lifecycle
counts running games and actively seated humans; the shared online state
notifies the room-list publisher only when those counts change. An existing
game's reservation is not a deployed human, and a departed command running
under AI does not count as one. Briefing rosters likewise describe who is
currently fighting, including the command being reserved, rather than every
human whose army the game remembers.
