# Slice 12: team planner, lanes and cohesion

**Decisions:** D2, D3, D9, D10′, D40, D41 (re-baseline at 12), D46, D48. **Needs:** 03 and 05. **Research:** [`../research.md`](../research.md).

## Contract it unlocks

AI players fight as concentrated combined-arms groups in their own lanes, so a team takes several objectives at once.

## Seam

- **Planner.** `crates/sim/src/skirmish_ai/team.rs` (new) provides `plan(side_frame, players, sites, memory) -> TeamPlan`. `TeamPlan` holds, per AI player:
  - a lane (`EntryIndex`);
  - a main objective;
  - a staging point.
- **How it plans:**
  - **Influence:** a coarse per-side grid of friendly minus visible-enemy strength, built from the side frame only and blended with its previous value.
  - **Covered objectives:** an objective counts as covered when the human's units are heading to it, read from `OwnUnit.goal` by `owner`.
  - **Assignment:** AI players are matched to lanes by brute-force over permutations, scored on distance and influence with an incumbency bonus. Ties go to the lower player.
  - **Memory:** incumbency is non-digested memory (D40).
- **Doctrine.** `BalancedDoctrine` reads its assignment through `PlayerContext.lane`.
  - It orders purchases into its lane, so slice 05 brings them in at that lane's entry.
  - It gathers its main group at the staging point, then moves it as one group on the main objective. This replaces today's per-unit "least-loaded objective" spreading (`mod.rs:662–694`).
  - Scouts, outposts and defensive positions may spread.
  - It keeps a complete army with its own trucks and AA (D2).
- **Tournament (D46).** The runner deletes its external doctrine pair (`runner.rs:82–100`). Every player runs as an in-battle AI, and acceptance counts come from `ai_receipts()`. Add 3v3 and 5v5 rows.
- **Re-baseline 2 (D41).** Re-bless the skirmish digests and fixtures, and name it in the commit.

## What the human can run or see

Tournament 3v3 and 5v5 reports, plus a 3v3 all-AI benchmark battle in the browser.

## Red tests first

- Three AI players on three entries get three different lanes and objectives.
- An objective the human is driving to is not assigned to an AI.
- The same observation and memory give the same plan.
- An incumbent keeps its lane when a challenger's score is only marginally better.
- After the opening, most of each AI's combat units are inside a group radius of its approach. Pin the behaviour, not a tuned number.
- Each AI's opening includes armour, infantry, recon, AA and a supply truck.

## Delegated

Grid resolution; scoring weights; the incumbency bonus; the group radius; staging-point choice. All of these are tuned against tournament evidence and stored in `fixtures/skirmish-ai.json`.

## Stays green

Non-skirmish digests. AI orders are never refused (`ai_receipts()` shows zero refusals).

## Feedback that would change it

The user watching the 3v3 and seeing blobs that look silly, or lanes left empty.
