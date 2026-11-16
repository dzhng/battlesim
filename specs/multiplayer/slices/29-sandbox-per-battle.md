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


## As built: code half (2026-11-15)

`deploy/sandboxes.ts` implements the unchanged `BattleMachines` contract. Its
async factory sweeps game-labelled leftovers before the lobby listens. Each
start reserves capacity, creates with an idempotency UUID and a private finite
rental, polls usable state, uploads one archive through a headerless upload URL,
launches the standalone battle process, exposes its player/control port and
waits for an authenticated control ping. Ambiguous create replies replay the
same request; a final ambiguous failure reconciles by a nonsecret allocation
label. Refused or timed-out starts reject with a generic error and reclaim any
known allocation. Service exception bodies never become diagnostics.

Stop is idempotent and returns immediately. It retrieves command stderr before
sandbox shutdown, removing the app credential and the same credential-shaped
hex strings slice 28 removes. It then stops, polls for archival and deletes;
transient cleanup failures retry within a finite bound. Startup attempts every
matching leftover even if one cleanup fails, and refuses to serve if a stop
cannot be acknowledged. The rental remains the final bound if service access
is lost. Other labels are untouched. The game label assumes one lobby per
workspace; restarting the lobby ends its old battles.

`deploy/settings.json` owns provisional capacity (8) and rental (10,800
seconds), with their rationale. The host entry passes them to the provider.
`src/lobby.ts` owns shared startup; `src/main.ts` supplies local children,
`deploy/lobby.ts` supplies rented machines. The battle entry reads `HOST` and
`PORT` with local defaults, reports readiness over IPC when there is a parent,
and otherwise needs only its control listener. Local children explicitly use
loopback and an ephemeral port instead of inheriting their lobby's port.

`http/controlProtocol.ts` owns the control timing: a 12-second running silence
lease, a one-second ping interval and a four-second attempt timeout. Any
successful control answer renews the lobby's lease; a failed attempt does not
end a battle. Preparation retains its separate bounded 120-second live request.
The battle side retains its lease check and immediate local parent-disconnect
shutdown. Twelve seconds allows a several-second stall and preserves slice
28's existing tests requiring dead-process detection inside fifteen seconds.
The real cross-ocean measurement is still outstanding. Transport drops are
retried by the existing resumable reader; a terminal protocol error still fails
the battle, rather than silently losing its presence and ending events.

The deploy dry run archives committed deployment code with the existing source
and data, uses the site's exact simulation bytes, and prepares the hosted lobby
and a battle archive. The explicit push adapter accepts an injected runner,
creates remote folders first, takes the workspace from the operator, requests
service capability, lets the host choose the port, and updates/restarts an
existing app. Its command construction is checked locally against the installed
CLI endpoint contracts. No push or real deployment belongs to this code pass.

The local rehearsal runs the actual deploy CLI against a static site with the
normal identity/simulation layout, then runs the resulting hosted entry against
the stand-in service and seats a player on a separate battle process. Provider,
lease, process, multiplayer-domain and local-scene evidence, deliberate failing
mutations and remaining manual operations are recorded in the ignored
`throwaway/multiplayer/slice29-report.md`. Host latency, battle-length memory,
cost, plan limits, public-route identity and two-network play remain the parent
agent's half of this slice; the slice is not marked complete by the code pass.

### Corrected at integration, against the real service (2026-11-15)

The code half was written against a stand-in. Three things only the
service could show, each checked with a scratch sandbox or a read-only
command, and each now what the stand-in does too:

- **A sandbox made without snapshots is erased by its stop.** About nine
  seconds after the stop it answers "not found"; it never reports
  `archived`, and there is nothing to delete. The service's list keeps a
  row for it, in its last live state. The provider now treats "not found"
  as gone. As written it would have asked about each such machine every
  second for three hours, and a lobby restarted after any battle would
  have refused to start.
- **The host's command-line tool:** its app list is `{ items, nextCursor }`
  at the top level, and registering an app takes the slug as a flag.
- **The push no longer makes the app public.** Who can reach a lobby is its
  operator's own step, on the first deploy and on every later one.

Also confirmed: a file uploaded with a relative path and a command with no
directory both use the same folder, so the fixed start command finds the
archive; the battle server's error output can be read until the stop and
not after, so reading it first is required, not a nicety.

## Deployed and measured (2026-11-16)

The lobby runs on the temporary host as the app `battle-lobby` in the
`armata` workspace, public, built from commit `2ad479413`. Its address is
not in the site: a development page reaches it with `?server=`. Each
battle below ran on a sandbox the lobby rented and gave back.

| Measured from California | Result |
|---|---|
| A room: Start to a ticket (a one-against-one on the small map) | 10 s, twice |
| The same in two real browser pages: Start to both playing | 24 s, with each page building the map and loading |
| Frontline: Play on an empty server to a briefing (a five-a-side on the largest map) | 57 s |
| Publications a second, room and Frontline | 30.1 |
| Gap between publications, 95th percentile | 35 to 39 ms; one over 100 ms in three runs |
| An order's round trip to the battle's sandbox | 179 to 187 ms at the median, 197 at the 95th |
| A lobby request (create, ready, start) | about 330 ms on the public route, about 100 on the private |
| The sandbox after its room emptied | gone when the empty room's minute ran out |

**O6 is answered for the route:** an order takes about 180 ms from
California, against 300 on the host's own public route, and the battle
keeps its tick rate. How it feels with someone far away is still slice
19's to judge.

**What the deploy showed:**

- **The host starts an app before its files have all arrived,** and says
  it did not come up. The push now asks again until it is up.
- **The app's own credential can rent sandboxes.** Nothing but the
  `services` capability was needed.
- **Play on an empty server is slow: 57 seconds.** A new Frontline game may
  draw several maps before one is admitted, and each refused draw now
  costs a whole machine: rent, upload, start, refuse, give back. Letting
  one machine try the next draw would take most of that away. Not done.
- **The battle server's archive is 10 MB,** nearly all the simulation, and
  is uploaded uncompressed for every battle.

**Still open here:**

- **The machine's size is settled: 8 GB.** On the smallest sandbox (2
  vCPU, 4 GB) a five-a-side of ten AIs used 0.5 GB at five minutes and 3.0
  GB at 42, when a side won; it grew about 65 MB a minute throughout. An
  hour would not fit in 4 GB. The size is data in `deploy/settings.json`.
- **A sandbox is not always a fast one.** That same soak ran well below
  the earlier ones on the same size: from the fifth minute to the thirtieth
  it kept 47 to 67% of real time, where slice 01's sandboxes kept 95%.
  The machines differ (slice 01 saw two CPU models) and are shared. Expect
  a full AI battle to run slow on some of them. The CPU-built native
  simulation measured in slice 01 is the known remedy, and still deferred.
- **The plan's limits** on sandboxes at once and starts a minute: not
  probed. `capacity` is still the first guess of eight.
- **What a battle costs:** not read from the service. By its price list
  the smallest machine is about two cents an hour.
- **Whose address a wrong room code is counted against.** The public
  route's header for a client's address is still unread, so every player
  shares one count.
- **A room battle and a Frontline game each played to their end** through
  the deployment. Both were started and watched running; neither was
  played out.

## Promoted (2026-11-16)

Production serves commit `2c2328133` with the lobby's address in it. The
promotion found the lobby out of step (the site's own build of the
simulation differs from a local one), deployed it from the promoted
commit, and the lobby then stated the site's three hashes. Two browser
pages on the production site played a room battle: both in the battle 53
seconds after Start, most of it each fresh browser downloading the game,
and 30 ticks a second after.

