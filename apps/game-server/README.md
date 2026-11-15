# Game server

The server owns online battle authority. Each battle runs on its own thread;
HTTP owns sockets and credentials, and a seat lookup binds each request to the
player it may command. Neither a ticket nor the battle setup contains a browser
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
The page checks its commit and simulation hash at `/hello` before opening a
room. Local builds compare the checkout commit and the actual simulation file.

The local build snapshots its identity and runtime data beside the bundle, so
changing the working directory or rebuilding the checkout cannot change a
running bundle's identity. The [release preparation](deploy/README.md) uses the
site's simulation and an archive of its commit; it defaults to a dry run.

Rooms reserve battle capacity when created, before anyone gathers or readies.
The [service](src/rooms.ts) holds them in memory; the shared room model alone
owns their rules. Room and list streams use the battle transport's record
framing, with a current snapshot on each open. A room stream opened with a resume cursor must still belong to its member;
a removed player is refused `removed` instead of silently joining again.
Start prepares the room's skirmish on a battle host, then publishes each
member's own ticket with the admitted map identity. A ticket's callsigns map
contains public names indexed by battle player, never credentials.
The room link restores a returning member's seat while its battle runs.
A finished or failed battle deletes its room and frees the reserved capacity;
existing battle readers receive the final observation before the stream ends.
Preparation failures and crashes close room streams with `battle_failed`.

A dropped member keeps their seat and army, driven by AI until any of their
streams reopens. Controller changes belong to the battle's recorded history;
they change no lobby membership. Empty-room deletion disposes its host too.

[Presence](src/presence.ts) counts all of a browser's open streams together.
Its disconnect grace protects seats across stream cuts; a room's lifetime is
measured from its last connected human, including a room nobody ever opens.
Callsigns are assigned by `/hello` and held while connected or remembered by an
online mode, including admission still preparing a battle. Once both have
forgotten a browser, its name is available again.

The client-address trust boundary belongs to [deployment](deploy/README.md).

[Online state](src/online.ts) owns battle admission and presence across modes.
A lobby holds a slot before it starts; Frontline holds one while preparing or
running. Both register the same seat lookup and use the same battle transport.

[Frontline](src/frontline.ts) reserves a seat while the briefing builds the map,
then Deploy takes it. A new game's first commander owns their seat from tick
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

The referee view is server accounting, read directly from battle state. It is
separate from either side's fog-limited publication. Administrative seat events
and this view have no remote-connection or HTTP entry point.
