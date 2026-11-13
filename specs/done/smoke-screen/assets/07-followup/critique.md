# Slice 07 follow-up: a seen cloud over the fog veil

Two fixes to the drawn cloud, judged on the smoke lab (`smoke` scene) before
and after:

- a cloud the side is shown draws over its fog veil, from its own side and the
  enemy's, while the ground and units under it stay veiled;
- the drawn cloud stands at the law's first bloom step from its pop tick.

**Verdict: accepted.**

## Proof the change is real

Each `before-after-*.png` holds the before shot on the left and the after shot
on the right. The `mask-*` pairs are the fog-mask debug view, where black is
unseen, white is seen and grey is partly seen. Before the change most of the
cloud read black in both sides' masks; after it, the cloud reads grey or
white, and the ground beyond it stays black.

The scene's new check, "a cloud a side sees draws over its fog veil, from
either side", samples the middle and a ring half way out and half way up. It
failed before the change on 6 of 13 points from red's side and 10 of 13 from
blue's, and passes on all 26 after (`throwaway/smoke-screen/scene-*.log`).

A pixel diff ran over all 25 shots (`compare-screenshots`):
- **Unchanged:** the shots before the pop and after the fade (`red-before-pop`,
  `blue-before-pop-panel`, `look-grass-before`, `red-gone`) are identical, with
  a distance of 0.
- **Changed:** every changed shot is one with a cloud standing, and each one's
  mean brightness rose; the side-by-sides show the change sits on the clouds.
- **Biggest move:** `look-bubble`, the close shot from inside the cloud, with a
  distance of 0.19.

## Triage of the unprimed critique (the last check)

The critique was given the 25 after shots and 10 crops, with no history. The
receipts check found no unattributed quantities (it flagged only the summary's
own item numbers).

| Finding | Status |
|---|---|
| Thin puffs at a fading cloud's edge keep a faint hatch, and the arc of the unit's seen patch shows through. The same holds for faint hatching over clouds in an all-veiled view. | **Accepted.** A puff lifts the veil by what it covers, so where the smoke is thin, the veiled ground behind it shows through. This is the rule "the ground stays fogged". It is far lighter than before, when the whole cloud was hatched and split. |
| A hatched lip and the veil's outline beyond the billow's soft edge. | **Accepted.** That is unseen ground beside the cloud, and it stays veiled. |
| A rectangle of thinner smoke round a hull inside its cloud; the barrel crisp over the smoke. | **Known.** Recorded in slice 07: camera-facing puffs are depth-occluded by the hull's top. It is more visible now that the cloud is unhatched. |
| The first puffs read as dense white balls on the hull. | **Accepted.** This is the change asked for: the law conceals out to its first step at once, so the drawn cloud stands there too. It no longer grows from a point. |
| The squad's cloud under the canopy reads as a white cut-out between crowns. | **Seen and left.** The crowns occlude it, as before. It is whiter now because it is no longer veiled. |
| Slivers and stair-steps in the veil edge; the veil cuts through crowns; ragged mask edges. | **Existing fog geometry, not this change.** These shots' diffs are near zero away from the cloud. |
| The red marker disc hides most of the enemy cloud; the label backing smudges over white smoke; the event log covers a unit. | **Existing UI, not this change.** Slice 06 recorded the marker over the cloud. |
| Clouds cast no shadow; the wreck's smoke style differs. | **Existing,** and outside this follow-up. |
| `blue-popped` duplicates `blue-pop-panel`. | **Existing scene quirk:** both are shot on the same tick. |
