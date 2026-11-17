# Implementation decisions

The user chose fixed endpoint-derived layers, LowAir for lofted ground fire,
unchanged vulnerability, one-attempt missiles that remain live, and layer-aware
visibility for both paths and explosions. These are requirements, not agent
decisions. Implementation decisions are recorded here after each reviewed pass.

## Sound

### Retain the round's band on transient flight and burst evidence

- When: visibility pass.
- Decision: a round that hits within one tick may already be gone when the
  side's view is published. Its path and explosion keep the band alongside
  their existing event data, so a visible air-to-ground hit does not disappear.
  The alternative would try to recover the band from a removed round or expose
  new client fields for a decision the simulation already owns.
- Gap: the plan specified the visible result, not the lifetime of its evidence.
- Reach: publication shape and client readers stay unchanged; the added data is
  transient and derived from digested round state.
- Verdict: sound. One authority grants visibility before clients receive it.
- Confidence: high.

### Sample an airborne impact at the physical blast origin

- When: visibility pass.
- Decision: a ground collision can round just below the terrain. A sight ray to
  that exact coordinate is blocked even though the missile was visible before
  impact. Sight now samples the existing damage origin just outside the struck
  surface, while publishing the actual impact coordinate. Ground-band fog keeps
  its existing rule, and aircraft-crash ground explosions keep their behavior.
- Gap: the plan did not specify how to see a burst on a collision surface.
- Reach: damage and burst visibility share the existing physical origin; no new
  height threshold, sensor exception or tuning value is added.
- Verdict: sound. The regression demonstrated the exact floating-point failure.
- Confidence: high.

### Split the battle authority by the responsibilities it already owns

- When: preparation pass.
- Decision: command admission, observation publication, air lifecycle, physical
  bodies, scripts, setup, countermeasures and aftermath each live beside the
  authority. For example, publishing a side's view still reads the same state at
  the same point in the tick; its code now has a readable module of its own.
- Gap: the plan did not specify how to make the oversized battle file editable.
- Reach: the authority retains the public API and tick orchestration; no new
  behavior or compatibility path was added.
- Verdict: sound. Existing methods moved unchanged across ownership boundaries.
- Confidence: high.
