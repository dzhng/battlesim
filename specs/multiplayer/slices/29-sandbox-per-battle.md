# 29 A sandbox for each battle

**Stage:** Hosting. **Needs:** 28. **Carries:** decisions 68, 8, 67.
**Burns:** O6. **Replaces** slice 09's push.

## Contract

- **The lobby runs as an app on the temporary host,** public, at one
  address. This is slice 09's deploy step, corrected with what slice 09
  records as learnt.
- **Each battle runs in a sandbox the lobby starts.** A third provider of
  `BattleMachines` (slice 28) creates a sandbox with a finite lease, puts
  the server bundle on it, starts the battle server, opens its port to the
  public (a page on another origin cannot send the sandbox's access
  token), and gives back its address. Stopping stops and deletes it.
- **Only `apps/game-server/deploy/` knows the host and its sandbox
  service.** The provider lives there and the lobby is handed it at start.
  It calls the service with the credential the host gives the app, never
  one written down.
- **Nothing is left running.** A battle that ends stops its sandbox. A
  lobby that starts stops any sandbox it finds from an earlier life. The
  lease is the last resort.
- **`full` means no machine could be had:** the plan's limit on sandboxes
  at once, or on starts a minute.

## To measure and decide here

- **Starting a battle, as a player waits for it:** from Start or Play to
  the battle server being ready, from the lobby's own location. From this
  computer a fresh sandbox answered in about nine seconds before any map
  was built (slice 01).
- **The size.** The smallest sandbox ran the soak as fast as any, but the
  soak's memory was still growing at ten minutes. Run a Frontline-length
  battle on it and read the memory before choosing between 4 GB and 8 GB.
- **The lease's length,** against the longest battle Frontline allows.
- **The limits of the workspace's plan:** sandboxes at once and starts a
  minute. They become `capacity`.
- **O6:** an order's and a preview's round trip from this computer through
  the deployed route, and the feel of a room battle from two browsers.
  The sandboxes measured so far were all in France.

## Carried over from slice 28's integration

Slice 28 was built and proven between processes on one computer. Three of
its choices need a second look once a lobby and a battle are an ocean
apart:

- **One late answer ends a battle.** The lobby asks the battle server it
  is alive every two seconds and treats one answer slower than five as a
  crash; the battle server stops if it hears nothing for ten. Measure the
  link between the lobby and a sandbox and set these so a blip does not
  end a battle.
- **Why a battle crashed stays on its machine.** The lobby is told only
  that it failed; the reason is written to the battle server's own error
  output. A sandbox is deleted when its battle ends, so bring that output
  back first.
- **The control surface shares the players' port.** It is guarded by the
  battle's key alone. On a sandbox that port is public.
- **The remote host's raw seat routes serve only the contract tests.**
  Nothing in the lobby attaches a seat to a remote battle. Decide at the
  closeout whether the tests should reach seats the way players do.

## Checked about the service on 2026-11-15

Through the host's service route, with a scratch sandbox: labels given at
creation come back on the sandbox and filter the list; values given as
`env` are visible to commands run in it; `snapshots: false` is accepted; a
port opened as public answers with no token; a stop took about two minutes
to reach `archived` and four to settle. A fresh sandbox answered the game
server's `/hello` nine seconds after the create call.

## Run or see

A development page pointed at the deployed lobby with `?server=`: a room
battle from two browsers, each battle on its own sandbox.

## Verification

- The provider's own tests run against a stand-in for the service: a
  start that is refused becomes `full`; a sandbox whose battle ended is
  stopped; leftovers from an earlier life are stopped at start.
- A deploy dry run that reports what it verified and what it would send.
- On the host: `/hello` answers; a room battle and a Frontline game each
  finish; the workspace's list of sandboxes is empty afterwards; the
  charge for each battle is read from the service and recorded here.

**The user has said to deploy on the temporary host (2026-11-15). Ask
before** putting the lobby's address in the site and before promoting a
site build: that is what makes it public.

## Must stay green

Slice 28's tests and scenes on the local providers.

## Delegated

The lease; the bundle's way onto the sandbox; labels for finding
leftovers.
