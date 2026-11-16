# 05: Spike: merge a block's props into one mesh

An experiment. It ends in a go or no-go with evidence, and no production code.

## Question

After the far mesh ships, does merging a block's far-drawn props into one
simplified mesh, chosen by its error in pixels, save enough GPU time to
justify generated geometry and a new population? This is Nanite's idea applied
across instances instead of inside one mesh.

## Ladder: stop at the first rung that answers

**1. Ceiling.** In a scratch checkout, omit every far-drawn prop from the view
and the cast pass at the wide hold, and measure by
[the protocol](../README.md#measuring). The picture is wrong on purpose; the
number bounds any merge. If the win is inside the noise, or under about a
tenth of the wide frame, the verdict is no-go and nothing is built.

**2. Does merging collapse anything?** Take one dense block's far-drawn props
from the frozen scene, in world space, with their paints. Run the pinned
reference builder over them (meshoptimizer `clusterlod`, as
[three-nanite at `b730aac`](https://github.com/apekshik/three-nanite/tree/b730aac13ab0ea67b75214613eea56c14151f6e5)
uses it: 128-triangle clusters, groups of about 16, halve each group, locked
group borders, error that only grows up the tree). Reproduce the reference's
own behaviour on a connected test surface first, so a failure is ours and not
the port's. Then report triangles against error for the block. Separate
islands may not simplify across each other at all; if merged triangles are
not well under the instanced total at a one-pixel error, the verdict is no-go.

**3. Draw it.** Replace the block's instances with the merged mesh through
the existing model path and measure one interleaved comparison.

## What a go must also answer

A timing win alone is not a go. Each of these needs an answer with evidence:

1. **Side knowledge.** A merged block is valid only while this side knows of
   no hidden, replaced, shoved or destroyed member. Otherwise the whole block
   falls back to its members before the next frame, never to a hole and never
   by asking the simulation what the side has not seen. Measure how many
   blocks are still merged late in a battle.
2. **Shadows.** The reference reuses the camera's choice for shadows and
   drops casters outside the view; we cannot. Start with the members' own
   casters unchanged and count only the view-pass saving. A merged caster is
   a second experiment, if time remains.
3. **Per-instance paint and per-face fog** survive the merge.
4. **Identity and lifetime.** The art store keys by content; a generated mesh
   needs a key covering its members' art, placements and paints, and one
   owner in both clients.
5. **Build cost.** Maps are generated at run time, so merging happens at load,
   in Rust, in the browser too. Measure the time and memory, and confirm a
   simplifier builds for `wasm32-unknown-unknown`.

## What the user can see

`assets/block-merge-verdict.md`: the ceiling table, triangles against error,
the comparison if one was drawn, answers to the five questions, and go or
no-go. If go, a sketch of the follow-up: the owner, the invalidation rule and
a first slice someone could review. Any picture offered as evidence gets
[`compare-screenshots`](../../../.agents/skills/compare-screenshots/SKILL.md)
against the shipped game, then an unprimed
[`screenshot-critique`](../../../.agents/skills/screenshot-critique/SKILL.md),
then [`preview-shots`](../../../.agents/skills/preview-shots/SKILL.md) without
blocking.

## Stop

One working day, two block sizes at most, one Metro block and one small
adversarial fixture. At the stop, write what is known.

## Delegated

- Which block, which sizes, and how the builder is driven in scratch.

## Must stay green

Production is untouched. Delete the scratch checkout once the verdict is
written.

## Feedback that would change this slice

- "Merge walls by run, not by block": test it as a narrower arm. A wall's run
  as one prism may be the cheaper and more legible merge.
