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

Open `http://127.0.0.1:5173/online/dev` in two independent tabs or browsers.
The dev flag enables temporary two-seat entry; without it the endpoint is absent.
`PORT` chooses the server's listening port. A development page can name a
different local address with `?server=http://127.0.0.1:<port>`.

Dev entry is scaffolding until rooms own battle creation. It keeps one fixed
battle in memory, returns the same ticket to a returning tab, and refuses a
third tab. A restart discards it. The tab keeps its random credential in session
storage; duplicating a tab may duplicate that credential too, so open a fresh
tab when taking the other seat.
