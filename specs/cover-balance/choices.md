# Implementation choices

## Sound

### Equal weight for the six direct-fire comparisons — medium confidence

**When:** measurement protocol.

When rifle fire and HMG fire run for the same duration, the rifle squad’s eight rifles contribute far
more rounds than one HMG. Adding all their damage together would let the squad
decide most of the reported cover strength. Each weapon/distance comparison therefore first
computes its own reduction, and the six reductions get equal weight in the tier
summary. Every individual result remains visible. The plan named representative
aggregation without fixing these weights. This choice makes future balance
reports treat a short rifle engagement and a long HMG engagement as equally
important scenarios. Sound: it prevents high-volume cases hiding weaker ones.

### Frozen parity inputs keep their historical catalog — high confidence

**When:** wreck-category naming pass.

A frozen parity fixture is an old battle input and its expected result, used to
prove the native and browser implementations agree. Some inputs contain the
old wreck names and their own complete catalog. They keep those names together:
replacing a name in only the assertion or re-recording its expected result would
stop testing the original input. Current authored game data uses the new names;
no runtime alias was added. The plan required an atomic current-data rename but
did not explicitly separate frozen input bundles. Future changes must continue
to distinguish historical oracles from the current catalog. Sound: the old
fixture is self-contained and cannot leak an obsolete name into live authoring.

### Sound — isolate a building firing position (medium confidence)

- **When:** measurement pass.
- **Choice:** Building measurements use one soldier at a real garrison seat. When
  eight soldiers occupy different façades, the shooter's known center can land
  inside the house, so its guns refuse to fire. Calling that 100% protection
  would confuse targeting with surviving incoming rounds. Exterior obstacle
  rows still use eight soldiers, and the whole village includes real garrisons.
- **Gap:** The spec did not choose a building occupancy for the controlled report.
- **Reach:** These rows establish protection at an occupied firing position;
  they do not certify squad targeting across several façades. Repairing that
  targeting issue would change another rule and is left outside this tuning pass.
- **Verdict:** Sound: preserve targeting while measuring actual incoming fire.
- **Confidence:** Medium; the limitation must remain visible in the result.
