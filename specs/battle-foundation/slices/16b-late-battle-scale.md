# 16b — Late-battle scale: remains, planning and fog

**Status:** proposed by slice 16's verdict; not started. Needs the user's go-ahead: several options change outcomes or presentation. **Dependencies:** 16. **Milestone:** Village checkpoint follow-up (before slice 22's full battle).

## Contract and question

Slice 16 measured the architecture at 100 units a side. The live battle meets the frame budget, but the late state does not. What change makes a long battle with persistent remains sustain 30 Hz and readable frames, without capping rounds or erasing remains?

## Bottlenecks measured in slice 16

Each has evidence in slice 16's verdict.

1. **Remains travel and redraw every tick.**
   - Every publication carries every corpse the side knows.
   - The main thread decodes them and rebuilds the whole remains overlay each time.
   - In the late state (20,000 fallen, 2,000 wrecks) the browser runs the simulation at about 6 Hz, frames take about 440 ms, and the page holds about 800 MiB.
2. **Remains bury living units.**
   - At the overview distance, a fallen soldier is drawn at the same size and in nearly the same tint as a living one (unprimed critique: blocker).
   - Units at a 2.4 km overview are too small to find at all.
3. **Order waves stall the step.**
   - Each unit runs its own A* on the 2 m navigation grid.
   - About 160 simultaneous orders cost about 1 s in one tick.
4. **The fog sweep dominates the steady step.**
   - Every unit re-sweeps its full ray fan every 6 ticks.
   - That is about 23 ms per sweep at 100 units a side, after the outcome-identical speed-ups.
5. **The stress input's own limits.**
   - The authored late-state wrecks are static map props, so every side knows them, unlike wrecks made in battle.
   - The roster cannot reach validation.md's 6k–8k rounds per second at the fixture's fire rates: slice 16's peak is in its verdict.

## Options, each a decision

- **Remains:**
  - publish them as deltas into a client-side store, and draw them from a mesh built once and appended to;
  - use a level of detail by distance;
  - render remains flatter and darker than living units;
  - author late-state wrecks as battle-made wrecks, not map props.
- **Planning:**
  - plan once per group toward the shared goal, with formation offsets;
  - or give each tick a planning budget (a unit may wait a tick or two to start);
  - or a tighter octile heuristic (same cost, different tie-breaks).
- **Fog:**
  - cache a stationary unit's sweep (outcome-identical);
  - spread sweeps across the 6 ticks (fog up to 5 ticks older);
  - or sweep long ranges at a coarser step.
- **Fire intensity:** decide whether the 6k–8k rounds per second target belongs to slice 22's full battle rather than this roster.

## Verification

Re-run slice 16's `endurance` scene (production build) and `endurance_report` against validation.md's targets, with the same seeds. Replays and digests must stay green, and outcome changes must be named in decisions.md.
