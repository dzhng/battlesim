# 17 Chapter 7: The push

**Status: shipped 2026-11-19.** Chapter 7's beats, defenders, solution and
coach steps; `match_result` and `open_shop`; the last-chapter admission rule;
the walkthrough's human-wins assertion; the closing step over the result.
Evidence: `director_facts` and `director` (a result answered on its tick, an
open-ended director refused), the walkthrough to blue's Victory (1384 game
seconds), `coach.test.ts`, and the scene's Victory frame. Deviations:
[decisions.md](../decisions.md#slice-17).

**Unlocks:** the whole tutorial, from the menu tile to Victory. See
[story.md](../story.md#chapter-7-the-push). It follows the chapter slice shape
in [13](13-chapter-3.md#shape-of-every-chapter-slice-1317).

**New vocabulary:**

- `Trigger::MatchResult`;
- an `Offer` that restores the faction's whole shop.

**Admission rule enabled:** the last chapter must end on a `MatchResult` beat.
This removes slice 08's named relaxation, and the walkthrough's
human-side-wins assertion turns on.

**Red:** defenders on `bridge` and `hill`, which the director moves onto the
flags as blue approaches. That gives Contested a moment. Their condition is
worn enough that the solution wins.

**Closing:** after Victory (the existing `MatchResultOverlay`), the coach's last
`continue` step plays `c7.guide`, which points to Esc → Guide.

**Gates:** in addition to the chapter shape, the walkthrough reaches blue
Victory with every beat fired. From here on it is the standing regression gate
for the scenario. Shoot the Victory frame with the coach's closing step;
screenshot-critique last; show it inline (non-blocking).

**Delegated:** how many defenders, and on which flags they make their stand.
