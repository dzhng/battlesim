# 09 Identity, build check, deploy

**Stage:** Foundation. **Needs:** 08. **Carries:** decisions 5, 8, 17, 46, 59,
63, P9, P10; landmine 7. **Burns:** O6 (measured here, judged at slice 19).

## Contract

- **Who you are:** a callsign and a browser id, both kept by the browser.
- **Which build may play:** only a page whose commit and simulation file
  match the server's.
- **Where the server is:** one committed address.
- **How it gets there:** one deploy step, the only code that knows the host.

## Seam

- **`apps/battle-lab/src/online/identity.ts`** (new): `browserId()`,
  `callsign()`, `setCallsign()`. The id is 128 random bits, made once. The
  callsign starts as a random one (`VIPER-41` in the mock), so the player is
  never asked. Both live in `localStorage`, read defensively.
- **`apps/battle-lab/src/buildIdentity.ts`:** gains the full SHA-256 of the
  served simulation file beside the short fingerprint it shows today.
- **`POST /hello`** in the server: compares the page's `BuildIdentity` with
  the server's `build.json`, and returns the callsign with `#2` added on a
  clash. A callsign has a length limit and a character set, nothing more.
- **The site build** publishes `build.json` (`{ commit, sim }`) and the
  simulation's two files, byte for byte, under `dist/sim/`.
- **`apps/battle-lab/src/online/server.json`** (new): the address. A page on
  localhost uses the local server; a dev build accepts `?server=`.
- **`apps/game-server/deploy/`** (new), run as
  `bun run deploy:server --site <site url>`: fetches `build.json` and the
  simulation files from that site, checks the hash and that the commit is the
  checkout's, bundles the server, adds the game data of that commit, and
  pushes the folder. `bento.mjs` there is the only file that names Bento,
  including the header its proxy uses for a client's address (P25).

## Run or see

The dev battle from slice 08, played through the deployed server's public
address.

## Verification

- `/hello` tests: a page on another commit or another simulation file is
  refused `build`; two players with one callsign get `name` and `name#2`.
- A deploy dry run that stops before pushing and reports what it verified.
- A page refused by `/hello` says online play is not available on this
  build, and offers nothing online.
- **O6:** through the public route, measure an order's round trip and a
  preview's from this machine, and play the dev battle from two browsers.
  Record the numbers here. The user's verdict on the feel comes at slice 19.

**Ask the user before** promoting a site build for this, before deploying,
and before making the server's address public. If the user prefers to wait,
do everything up to the push and continue with slice 10.

## Must stay green

The build stamp the menu already shows.

## Delegated

How the build emits `build.json`; callsign length and characters; the random
callsign's word list.

## Results

Not run yet.
