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
6. Delete the probe app `mp-probe` from the `armata` workspace, and the
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
