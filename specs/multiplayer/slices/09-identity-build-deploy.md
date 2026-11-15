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

## As built: the local half (2026-11-15)

Everything except the push and the measurement through the public route.

- **Identity** (`apps/battle-lab/src/online/identity.ts`): the browser id is
  16 random bytes in hex and the callsign starts as a random one (one of
  eight words and two digits, such as `VIPER-41`). Both are kept in
  `localStorage`. If storage is missing, full or holds garbage, the page
  still has a usable id and callsign for that visit. This replaces the
  tab-scoped id of slice 08.
- **Callsign rule** (`packages/multiplayer/src/callsign.ts`), one owner for
  page and server: at most 24 characters from letters, digits, space,
  underscore and hyphen. A name is cleaned to fit, not refused; `#` belongs
  to the server's clash suffix.
- **`POST /hello`**: a page whose commit or simulation hash differs from the
  server's is refused `build`. A welcomed page gets its callsign back, with
  `#2`, `#3` and so on if another browser holds it. The same browser asking
  again keeps its own.
- **The server's own build identity** is a `build.json` written beside its
  bundle when the bundle is made, with the game data and the simulation file
  it was built from. The server reads those, never a live checkout.
- **The site build** publishes `build.json` and, under `sim/`, the
  simulation's files byte for byte (the glue's panic-reporting snippet
  included, which the server's bundle needs).
- **The address** (`apps/battle-lab/src/online/server.json`) is empty until
  a server is deployed. A page on localhost uses the local server; a
  development build takes `?server=`. A page with no address makes no
  request and says online play is not available.
- **`bun run deploy:server --site <url>`** is a dry run unless `--push` is
  given. It fetches `build.json` and the simulation from the site, refuses
  if the commit is not the checkout's or the simulation's hash is not the
  one `build.json` names, builds the server from that commit's source and
  data, and lists every file it would push. `deploy/bento.mjs` is the only
  file that names the host.

Not done, and why:

- **The push has never been run.** `deploy/bento.mjs` was written from the
  host CLI's help text alone. Expect to correct it on the first real deploy.
- **The header the host's proxy uses for a client's address** (P25) is not
  filled in: it can only be learnt from the host.
- **O6** (round-trip times through the public route) needs the deployed
  server.
- **The deploy command starts the server with its dev flag,** because the
  dev battle is the only way into a battle until rooms exist. A deployed
  server therefore offers the dev battle to anyone who has the address.
  Decide at deploy time whether that is acceptable for the measurement, and
  drop the flag in slice 18.
- **Callsigns are remembered for the server's lifetime** and never
  released, so a name stays taken after its browser has gone. Slice 15,
  which knows who is present, owns releasing them.
- The page a refused browser sees is the shared failure panel, headed
  "Aborted", with the sentence "Online play is not available on this
  build." and a way back to the menu. It is honest and plain; slice 16's
  multiplayer screen is where it gets its proper place.

## Results

Not run yet.
