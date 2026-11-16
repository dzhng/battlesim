# 27 Closeout

**Stage:** Frontline. **Needs:** every other slice.

## Contract

The plan is finished, proven once in full, documented where each concept
lives, and archived.

## Steps

1. Pull what the full run needs from large-file storage first: the audio,
   the roster and infantry sources, the scene baselines and the city.
2. **The one full run.** Start it in the background with a visible sign of
   progress, and leave that checkout alone until it ends. Fix what it finds,
   each unrelated failure in its own commit.
3. Sweep for leftovers with
   [refactor-change](../../../.agents/skills/refactor-change/SKILL.md): the
   dev battle scaffolding (slice 18 should have removed it), any second copy
   of a seat or lobby model, any page-side sequence counter, any server code
   that names the host outside `apps/game-server/deploy/`.
4. Run [review](../../../.agents/skills/review/SKILL.md).
5. Docs, with [write-docs](../../../.agents/skills/write-docs/SKILL.md):
   `development.md` gains where the server, the wire contract, rooms and
   Frontline live, and the two seat events if they are deliberate rules. The
   root README says how to run the server beside the site. Each new folder
   has a README written for the next agent.
6. Delete the probe apps `mp-probe`, `mp-soak` (the host soak of
   2026-11-15) and `addr-probe` (the address header check of 2026-11-16) from the `armata` workspace, with the user's word, and the
   scratch under `throwaway/mp-probe/` and `throwaway/multiplayer/`.
7. **Ask the user** before promoting and deploying Frontline, as in slice 19.
8. Archive the plan with
   [close-spec](../../../.agents/skills/close-spec/SKILL.md): move it to
   `specs/done/` and rewrite it from a ladder into the rationale that should
   outlive the build.

## Verification

- The full run is green, and its output is linked from this file.
- Every open item in the README is closed or named in decisions.md with what
  it now means.
- Every row of decisions.md part 3 marked ★ has the user's answer beside it,
  or a note that they were shown it and let it stand.

## Results

Not run yet.

Notes for the run (2026-11-16):

- Staging's merge of that day brought buying a squad aboard a carrier. The
  online purchase preview dropped the carrier; it now carries it (wire,
  server, battle host), with tests.
- The three result-screen pictures (`ui/report-*`) are re-approved: the
  only difference from the approved ones was the headings, "YOUR SIDE" and
  "ENEMY". The main menu's picture is re-approved with the Multiplayer
  tile, whose art the user approved on 2026-11-16.

## Results (2026-11-16)

The one full run, on the branch with staging merged (`1ac47e628`) and the
fixes that followed it.

- **Format, lint, typecheck, the WebAssembly build:** pass.
- **The Rust workspace, run to the end:** 165 test groups pass. One
  failed, the digest pins in `seat_replay`, which matched staging's own
  values and are retired (decisions, part 4).
- **The web suite:** 1,856 of 1,857 pass. One Frontline test timed out
  inside the whole suite and passes alone; it has its neighbours' hang
  watchdog now.
- **The scene suite:** 1,669 checks pass. Four pictures are re-approved
  (above). Two failures are staging's own, shown by running the same
  scenes on a plain staging checkout the same day, and are left to the
  sessions that own that work: `allies` (your tanks do not reach your
  ally's force) and `frame`'s `river` picture (3 pixels).

What the run found that the multiplayer tests could not: a release-order
test that runs the real promotion script; a stale expectation of the
authority's opening request; a site build that threw where git cannot be
run; a function name the math-owner check reads as a matrix helper; a
scene helper that imported a module the scene runner cannot load, which
stopped the whole scene suite. All fixed.

Not rerun after those fixes: the whole of each suite. The affected tests,
the lint and the scene suite were.

## Closeout sweep (2026-11-16)

Step 3 was executed locally, without contacting a host, deploying, promoting
or pushing. Evidence and exact commands are in
[`throwaway/multiplayer/sweep-report.md`](../../../throwaway/multiplayer/sweep-report.md).
The rest of this slice's operational closeout is outside the sweep.

1. [x] Removed lobby-side seat attachment and private raw seat routes,
   capacity-only online state, service factory injection, battle accessors
   and slot lookup fields. Tickets require their battle server's address.
   Service scenarios allocate the existing in-process HTTP machines; player
   commands and admission use player routes. Runtime observation taps retain
   disconnected-army and replay assertions without pretending to be presence.
2. [x] Publication payload encoding and decoding have one stream-package owner.
3. [x] Browser identity format, header and diagnostic scrubbing have one
   multiplayer owner. Browser-taking routes validate the format.
4. [x] Return navigation, typed public notice state and chosen Frontline
   departure each have one owner in `onlineSession.ts`.
5. [x] The room model returns refused-join codes directly.
6. [x] Request refusals, the seat client, the two seat types, room-battle
   notice, general online failure wording, HTTP refusal and hosting owner
   now have names matching their scope. Consumers and docs moved together.
7. [x] Shared lighting validation, bounded index draws and identical removal
   notices. **Left:** seed draws differ in width and random-source consumption
   (rooms use two words; Frontline uses one clamped word). The differing
   existing closed-room and map-progress sentences remain, since changing
   either would change visible copy outside item 9.
8. [x] Removed the map-cache setter and result wrapper fallback; tests use
   ticket navigation. Removed the extra generator-only simulation import and
   field from server data. Machines expose one address for player and
   authenticated control routes. Catalog admission still legitimately loads
   the simulation to resolve the game's rules.
9. [x] Wrong-build and unreachable hello failures reach the existing unavailable
   notice as two safe sentences. Regression observed red before green;
   both states captured at 1280×720.
10. [ ] **Left under the no-behavior-change stop rule:** the shareable parser
    distinguishes passive AI (`p`) from active AI (`a`), while a lobby AI
    occupant projects only to active AI. The position-based parser model and
    its projections remain until that link contract is deliberately settled.
11. [x] A stale-tab sequence refusal settles the batch's orders to the page and
    cancels retries. Regression observed red before green; HTTP 409 remains.

No simulation source, digest, replay or parity fixture was changed. No
baseline was re-approved. The report records final verification and any
remaining failures, including the explicitly excluded math-owner failure.
