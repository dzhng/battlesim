# 31 A build is what both ends share

**Stage:** Hosting. **Needs:** 30. **Carries:** P9, P10, decision 68.
**Changes** P9.

## Why

The lobby admits a page only when the page's commit and simulation file are
its own (P9). That was the simplest honest rule while one person deployed
both. It does not survive how this repository ships: every push to staging
is a new commit, sessions promote the site several times a day, and after
each one the live lobby would refuse every player until somebody deployed
it again, whether or not anything the two ends share had changed. The user
asked on 2026-11-16 for the site to point at the lobby; this has to hold
first.

## Contract

- **A page may play on a server when they agree on what they share,** and
  on nothing else: the simulation's bytes, the documents both ends build a
  battle from, and the multiplayer wire. A commit that changes none of
  those is the same build to the lobby.
- **`BuildIdentity`** states those three, each as a hash made from the
  files themselves, by the build that made the page and by the bundle that
  made the server, with one owner for which files count. No number is
  raised by hand. The commit stays in the site's `build.json`, as the
  record of which source the site was built from, and is not compared.
- **`/hello` refuses `build`** exactly when one of the three differs.
- **The deploy step deploys the commit the site names,** whatever the
  checkout holds: it builds the server from that commit's source and data
  and the site's own simulation bytes, as today.
- **Promotion keeps the lobby in step.** After `promote:main` has promoted
  a site, it asks the live lobby whether it would admit that site's build.
  If not, it deploys the lobby from the promoted commit and says so; if
  that fails it says so loudly, with the one command that repairs it. A
  promotion that changes nothing shared leaves the lobby, and every battle
  in progress, alone.
- **The lobby's address is in the site** (`server.json`), and which app
  and workspace the lobby is live in is data under `deploy/`: only that
  folder knows the host.

## Seam

`packages/multiplayer/src/wire.ts` and [wire.md](../wire.md); the site
build's identity output and the page's `buildIdentity`; the server's
bundle and `http/hello.ts`; `apps/game-server/deploy/`; `tools/promote-main.mjs`,
which may call the deploy step and must not learn the host's name.

**Slice variable:** what makes two builds the same. Out of scope: letting
battles survive a lobby deploy.

## Run or see

Two commits apart that differ only in a README: a page of one plays on a
server of the other. Two that differ in a unit's stats: refused.

## Verification

1. Tests of the rule: builds that differ only in files neither end shares
   are admitted; a different simulation, a different shared document, and
   a different wire are each refused `build`.
2. The page's identity and the server's are made from the same list of
   files: a test that fails if one side's list gains a file the other
   lacks.
3. The deploy dry run for a commit that is not the checkout's.
4. The promotion step against stand-ins for the site, the lobby and the
   deploy: in step, nothing is deployed; out of step, the deploy runs for
   the promoted commit; a failed deploy is reported and does not undo the
   promotion.

## Must stay green

The multiplayer domain; the menu's build line.

## Delegated

How the three hashes are made and named; how the promotion asks the lobby.

## As built (2026-11-16)

`BuildIdentity` is `{ sim, data, wire }`. The build-time owner at
`packages/multiplayer/buildIdentity.ts` defines the simulation file, the shared
battle documents (disk copies and embedded JSON), and all multiplayer sources.
WASM keeps raw SHA-256; document and wire sets hash sorted paths, byte lengths
and exact file bytes. Vite fixes data/wire into the page; the page still hashes
the simulation it actually loads. Development restarts Vite when shared
inputs change, including wire-file addition/removal, so its bootstrap cannot
keep stale hashes after HMR. The bundle uses that same owner. The menu and
site build record retain source provenance outside admission.

`/hello` compares exactly those hashes through the shared `sameBuild` predicate.
The new credential-free `GET /build` returns only them and changes no player
state. It was chosen over a synthetic `/hello`, which would reserve callsigns
and presence just to check deployment compatibility.

Release preparation resolves the site's named commit locally, archives it and
runs its own bundler with the site's exact simulation files. Both artifacts
must match the site before upload. App/workspace belong to archived deploy
settings, with no caller/environment alternative. The committed page address is
populated; localhost and development overrides keep their existing behavior.

Promotion checks through `deploy:server --check`, then uses `--push` only when
needed. Both calls pin the immutable site URL and promoted commit. Failure
leaves the promoted site/main record in place, prints one repair command and
exits nonzero. A failed main-branch record still checks/synchronizes the
already-promoted site and retains a nonzero result. Dry runs also check, and an already-promoted approved run repairs
the lobby. Host details remain in the deploy boundary.

Local verification, deliberately broken behaviors, exact commands, decisions
and shipping steps are recorded in `throwaway/multiplayer/slice31-report.md`.
No external service, production site, host CLI, push, promotion or deploy was
used. The multiplayer scene is **NOT RUN**: its nonblocking lock attempt found
the shared GPU lock already held, so no scene or lock holder was disturbed.
Menu/refusal visual verification remains at integration. No simulation, digest,
replay or parity fixture changed.

### Corrected at integration (2026-11-16)

- **Promotion reads the production site, after promoting.** As written it
  read the staged deployment's own address, which asks for a sign-in (it
  answers with a redirect), so every promotion would have ended by saying
  the lobby could not be checked. A dry promotion now says the lobby is
  checked once the site is promoted, and checks nothing.
- **Promotion still records `main` as it did:** whenever production serves
  staging, with or without `--yes`. A lobby deploy still needs `--yes`.
- **The hosted lobby is told the client address header by the host
  settings,** and a local lobby is told none. The environment variable for
  it is gone.

Known and left: the wire's hash covers `packages/multiplayer/src` only.
A change to how a lobby becomes a battle request
(`packages/skirmish-setup`) or to map preparation is caught later, when a
page's map does not match the server's, not at `/hello`.

