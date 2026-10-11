# Choices ledger

## Sound

- **Ordinary orders are the policy boundary.** `BalancedDoctrine` emits the
  same `Order` values a player uses. The battle authority still checks faction,
  budget, cap, route, target, and objective rules, and replay records accepted
  commands. This keeps one outcome owner and lets a future policy implement the
  same `SkirmishPolicy` contract without a parallel economy.
- **Weapon data admits counters.** A helicopter sighting is answered by
  inspecting actual catalog mounts and weapon rows for the `low_air` target
  layer. A ground-only weapon is excluded even if its role name sounds useful.
  This is the general rule that prevents the original infantry-only failure.
- **Randomness is bounded and seeded.** Each decision consumes one draw from a
  stream salted by battle, side, and controller identity. The draw delays a
  response or nudges near-equal scores; hard capability and legality filters
  remain unchanged. Equal seeds repeat and a seed sample can vary choices.
- **Doctrine state is compact.** Low-air evidence, observed own-unit loss, and
  maximum dwell drive transitions. Force-shape states affect opening purchases,
  while transient pivot state stays out of digest/publication state.
- **Recon uses an ordinary move at a standoff.** The first post is interpolated
  along the entry-to-objective approach from published sight range and faces the
  objective. A side-scoped route query can reject it, after which the normal
  objective order remains the fallback.
- **The tournament is external.** Two controllers call observation, route,
  accept, and step directly. Generated rows pin request inputs and run through
  `PreparedMap` and `admit_skirmish`; saved rows retain deliberate edge cases.
  Reports preserve refusals, timeouts, no-progress outcomes, map/site identity,
  replay parity, and deterministic digests.
- **Default self-play is a real fresh match.** The example runs one generated
  mixed map at the smallest existing skirmish tier with a clock-derived seed;
  `--seed` makes it reproducible, `--max-seconds` supplies a safety cap, and
  `--corpus` keeps the deterministic sweep separate from interactive
  playtesting. The run ends on the simulation's terminal result when one is
  reached.
- **Stall detection follows observable motion.** A tournament run is stagnant
  only when objectives, contacts, unit counts, and published unit
  positions/goals all remain unchanged for the configured window. This keeps a
  long legal approach from looking like a deadlock.

## Deferred by design

- **Richer doctrine evidence:** objective-progress scoring, detailed casualty and
  logistics loss models, and a full task-group/intent layer are future work.
  The current state machine is intentionally smaller than the authored graph
  because duplicating order ownership would make behavior harder to audit.
- **Sight candidate search:** the policy uses published sight range and route
  legality today. Shape, concealment, and multi-post line-of-sight scoring can
  be added behind a side-scoped query without exposing true world geometry.
- **Instruction accounting and policy matrix:** replay/digest and gameplay
  evidence are shipped; instruction-cost reporting and multiple production
  policy matchups remain future harness extensions.
