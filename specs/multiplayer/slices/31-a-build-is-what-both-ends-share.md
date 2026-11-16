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
