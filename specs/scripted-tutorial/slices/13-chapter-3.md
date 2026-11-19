# 13 Chapter 3: The crossroads

**Status: shipped 2026-11-19.** Chapter 3's director beats, forces, zones,
solution and coach steps; the vocabulary in `contract::director` and
`battle/direct.rs`; the dev loop (`Battle::chapter_start`, `scenario_chapter`,
`&chapter=…&beat=…`); chatter's hold under narration. Evidence:
`crates/sim/tests/director_facts.rs`; the walkthrough plays chapter 3 and
starts there; the `scenario-battle` scene plays every later UI step from the
dev loop; `web/tests/battleAudio.test.ts` (no answer over the narrator).
Deviations: [decisions.md](../decisions.md#slice-13).

**Unlocks:** transport, garrison, suppression, attack-move, show orders, capture
and contest, taught in the battle. See [story.md](../story.md#chapter-3-the-crossroads).

## Shape of every chapter slice (13–17)

1. **Director vocabulary first.** Each new trigger or action is a typed variant
   with a test in `crates/sim/tests/director.rs` on **test units**, never on the
   scenario.
2. **Content.**
   - forces with lopsided `UnitCondition` and authored engagement;
   - zones;
   - beats (a milestone, audience blue, for every coach `beat` gate);
   - red cues as `DirectedAi` orders;
   - recovery beats for any unit the player could lose;
   - solution beats;
   - coach steps with their `line` ids, marks, highlights and camera.
3. **Narration.** A step whose line text changed is re-rendered through the
   slice 10 pipeline and judged.
4. **Gates.**
   - The walkthrough runs through this chapter. It already iterates every
     chapter, so a chapter adds no assertion code.
   - Coach validation.
   - A `tutorial.mjs` step for each UI-gated step the walkthrough can't reach,
     using restart (12) to start at the chapter.
   - Shots of the chapter's steps, screenshot-critique last, shown inline
     (non-blocking).
5. **`pause` steps** (from 11): the first chapter whose tour flies over a
   live battle adds the step's `pause`, holding the offline battle through
   the session's `SimClient.pause()` the way the pause menu does.
6. **The dev loop** (moved from 12): start the browser battle at this
   chapter's opening. The walkthrough's replay can't be resumed into the
   shipped scenario: the solution adds beats and hands the human seat to the
   director, so its scenario digest differs. Build the loop on the solved
   scenario itself instead. `Battle::resume` from an empty replay plays the
   director's solution to the chapter's tick. The seat goes back to a person
   there, and the loop stops the solution's later beats from steering the
   person's units (or drops the solution past the chapter).
7. **Iterate from the chapter's start** with restart's dev loop, never by
   replaying earlier chapters by hand.

## This chapter

**New vocabulary:**

- `UnitFact::Inside(host)`: aboard a carrier, or garrisoned;
- `UnitFact::Suppression(tier)`;
- `UnitFact::AttackMoving`;
- `Trigger::Objective { id, is: OwnedBy | Capturing | Contested }`.

**Red:** the `outpost` squad (worn, fire at will, so it can be suppressed and
pinned) and the `counter` squad (worn), which the director sends into objective 1
after blue captures it, to teach Contested.

**Delegated:** exact `UnitCondition` values (lopsided, not tuned toward a stat),
zone outlines.

**Also owned here (moved from 10):** a narration line ducks unit chatter. This
is the first chapter with combat chatter under Lantern's lines. Narration and
chatter share the voices bus, so the chatter's arbitration holds new lines (or
dips them) while a narration line plays. The mix owner sets how much.
