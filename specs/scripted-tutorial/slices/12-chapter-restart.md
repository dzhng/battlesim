# 12 Chapter restart

**Unlocks:** "Restart chapter" from the Esc menu, by deterministic replay. It is
also the dev loop for chapters 3–7: each chapter is iterated by jumping
straight to its start.

**Contract (C11):**

```rust
// crates/sim/src/battle/replay.rs
impl Battle {
    /// `replay`'s battle replayed to `tick`, then live: from `tick` on it takes
    /// input and runs its AI seats as a played battle does.
    pub fn resume(setup: &ScenarioDefinition, replay: &Replay, tick: Tick)
        -> Result<Battle, ResumeError>;
}
pub enum ResumeError { Replay(ReplayError), AiMemory { player: String }, Beyond { tick: Tick } }
```

- **Built on the existing replay.** The accepted log, controller transitions,
  seat events and script trace are cut at `tick`. After it, `replaying` clears,
  each side's and player's sequence counters continue past the prefix, and the
  battle is live. The director reruns from setup, so its state at `tick` is
  exact.
- **Refuses `AiMemory`** when any `ActiveAi` seat played before `tick`, because
  its memory is outside the digest. The tutorial has only human and `DirectedAi`
  seats, so it always resumes exactly.
- **The chapter's tick** comes from the published `ScenarioView`: the tick the
  chapter's opening was published.
- **WASM:** `BattleHandle::resume(scenario_json, replay_json, tick)`. Worker
  message `prepare-resume { battle, replay, tick }`, always a fresh worker (the
  scenario builder's lesson: one authority connection per battle lifetime).
- **Page:** `routes/battle.tsx` passes BattleView a "Restart chapter" pause-menu
  item for scenario battles; it asks first, like Restart. The loading screen
  shows replay progress (P2 decided whether that is needed). The coach
  re-derives its state from the resumed publication, and its UI signal log
  clears.
- **Dev loop:** open the walkthrough replay from 08, pick a chapter, and resume
  there with the human seat handed back to a person (a recorded controller
  transition at the resume tick).

**P2's verdict shapes it.** A plain replay to chapter 7 costs minutes
(see [decisions.md](../decisions.md#probes-p1-p2-p4-agent-2026-11-19)), so
restart is instant from a battle that is already there:

- **A standby battle per chapter.** When a chapter opens, the page starts a
  second worker that runs `Battle::resume(setup, replay, chapter_tick)` in the
  background, without publishing, and waits at that tick. "Restart chapter"
  makes that worker the live authority and drops the old one. The next
  chapter's opening replaces the standby. This spends one worker's memory for
  a smooth restart (the user's standing preference) and keeps replay as the
  only proof: no clone, no snapshot.
- **A fast-forward that publishes once.** The worker's resume runs its prefix
  without a publication per tick, then publishes the tick it lands on.
- Re-measure the real tutorial's resume cost in this slice. It is far smaller
  than P2's 30-a-side battle.

**Gates:**

- `crates/sim/tests/resume.rs` on a test map:
  - a resumed battle's digest at the tick equals the original's;
  - it then accepts a human order and diverges only by that order;
  - resuming after an `ActiveAi` seat played is refused `AiMemory`;
  - resuming past the end is refused.
- The walkthrough gains a step: after finishing, resume at chapter 2's start
  from its own replay and replay chapter 2's solution; chapter 2's beats fire
  again.
- The resume cost of the latest chapter is printed by the walkthrough as a
  loose tripwire (from P2), never a target.
- `tutorial.mjs`: restart chapter 2 from the pause menu lands on chapter 2's
  first step. Shoot it; screenshot-critique last; show it inline
  (non-blocking).

**Delegated:** where the chapter-tick lookup lives (a client helper or WASM),
the loading text.

**Must stay green:** `from_replay` and every replay test; the walkthrough.

**Feedback that changes it:** restart feels slow, which takes the P2 fallback
(hold a replay prefix per chapter in the worker).
