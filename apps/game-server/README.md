# Game server

The server owns online battle authority. Each battle runs on its own thread;
HTTP owns sockets and credentials, and a seat lookup binds each request to the
player it may command. Neither a ticket nor the battle setup contains a browser
credential.

For local play, from the repository root:

```sh
bun run server --dev
bun run --cwd web dev --host 127.0.0.1 --port 5173
```

Open `http://127.0.0.1:5173/online/dev` in two independent browser profiles.
The dev flag enables temporary two-seat entry; without it the endpoint is absent.
`PORT` chooses the server's listening port. A development page can name a
different local address with `?server=http://127.0.0.1:<port>`.

Dev entry is scaffolding until rooms own battle creation. It keeps one fixed
battle in memory, returns the same ticket to a returning browser, and refuses a
third browser. A restart discards it. Tabs share the browser credential kept in
local storage; separate profiles take the two seats. The page checks its commit
and simulation hash at `/hello` before asking for a ticket. Local builds compare
the checkout commit and the actual simulation file, including uncommitted
simulation builds.

The local build snapshots its identity and runtime data beside the bundle, so
changing the working directory or rebuilding the checkout cannot change a
running bundle's identity. The [release preparation](deploy/README.md) uses the
site's simulation and an archive of its commit; it defaults to a dry run.

Rooms reserve battle capacity when created, before anyone gathers or readies.
The [service](src/rooms.ts) holds them in memory; the shared room model alone
owns their rules. Room and list streams use the battle transport's record
framing, with a current snapshot on each open. Starting stops at the model's
`starting` phase until battle creation is connected.

[Presence](src/presence.ts) counts all of a browser's open streams together.
Its disconnect grace protects seats across stream cuts; a room's lifetime is
measured from its last connected human, including a room nobody ever opens.
Callsigns are assigned by `/hello` and held while connected or remembered by a
room. Once both have forgotten a browser, its name is available again.

The client-address trust boundary belongs to [deployment](deploy/README.md).
