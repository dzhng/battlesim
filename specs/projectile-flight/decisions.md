# Implementation decisions

The user chose fixed endpoint-derived layers, LowAir for lofted ground fire,
unchanged vulnerability, one-attempt missiles that remain live, and layer-aware
visibility for both paths and explosions. These are requirements, not agent
decisions. Implementation decisions are recorded here after each reviewed pass.

## Sound

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
