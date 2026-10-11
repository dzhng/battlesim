# Choices ledger

## User decisions

- **One balance policy:** ship one hierarchical balance policy; leave a clean
  extension seam for future policies.
- **Fallibility:** add seeded score noise and bounded delayed responses so the AI
  sometimes misses a counter without becoming arbitrary.
- **Doctrine failure:** pivot from evidence with a maximum dwell time.
- **Recon:** stage at the sensor envelope so scouts exploit sight range and shape.
- **Transitions:** use an authored transition graph, but simplify it during
  implementation if it does not earn its complexity.
- **Player experience:** pivots are visible through battlefield behavior;
  personality is emergent and has no selector or new HUD.
- **Compatibility:** hard cutover; no migration scaffolding.

## Answered by the agent on your behalf

The user opted in to resolve remaining choices using these confirmed
preferences: believable emergent behavior, physical observability, deterministic
evidence, and simplicity when machinery does not earn its cost.

- **AI-vs-AI hosting:** start with an external native host that drives both sides
  through public observation, side-query, accept, and step seams. Add a two-side
  setup controller only if the host probe proves it is needed for shipped play.
  This avoids changing the browser scenario contract before the tournament has
  evidence that it needs the larger surface.
- **Policy identity:** keep `BalancedDoctrine` as the only accepted production
  id. Future ids fail admission until a policy is intentionally implemented.
- **Trace:** collect doctrine/pivot/recon trace in the native tournament harness
  or a no-op controller callback; do not add it to observations, publication, or
  the battle digest.

## Rejected or deferred

- Live LLM/learned decision making is deferred. Offline tuning may use search or
  a learned ranking model, but runtime must remain deterministic and explainable.
- Dedicated air-defence units are deferred to their own mechanics/content slice.
- Player-selectable doctrine modes and faction-specific policy variants are out
  of scope for this feature.

## Implementation choices — slice 00

- **Policy seam and production name:** `SkirmishPolicy` is a Rust trait over the
  existing ordinary command output, and the shipped implementation is named
  `BalancedDoctrine`. The direct rename removes the old generic `SkirmishAi`
  name instead of leaving an alias: when the battle reaches its AI turn, the
  host still validates and records the same orders, while a future policy can
  replace the implementation without inventing a second command or economy
  path. The spec required an extension seam and one balance policy, but left
  the concrete name and compatibility treatment open. **Verdict: sound, high
  confidence.**
- **Seeded first variation:** a seeded controller starts its reinforcement
  rotation at a deterministic role offset. Two battles with the same seed make
  the same choice; different seeds can begin the otherwise identical rotation
  at a different role. This is a small legal source of personality while the
  later doctrine slice adds bounded score noise and delayed reactions. The spec
  required fallibility but left the first low-risk injection open. **Verdict:
  sound, medium confidence; revisit if tournament variance is too weak or too
  strong.**

