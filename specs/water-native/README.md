# Water on native (placeholder)

**Status:** placeholder. It collects the native-client work from
[water and wetlands](../water-and-wetlands/README.md) that is too complicated to
port in the same step as the browser. Nothing is listed yet, because no step
has been built.
**Updated:** 2026-10-27.

## Next Agent Prompt

You are keeping a running list for the native client
([`native/client`](../../native/client/README.md)) while
[water and wetlands](../water-and-wetlands/README.md) is implemented. That spec's
[decision 19](../water-and-wetlands/decisions.md#scope-and-acceptance) is the
contract:

- native must always start and draw water at least as well as before;
- each step ports to native what it reasonably can;
- what's left lands here, as one item per gap, with the step that left it and
  why it was hard.

Don't spend long on native in a water step. When an item lands here, say what
native draws instead (for example, the old water material, or no wakes). Load
[`renderer`](../../.agents/skills/renderer/SKILL.md) before any GPU work. When
the water spec closes, walk this list with the user, then run
[`write-spec`](../../.agents/skills/write-spec/SKILL.md) on what they want.

Update this section and the list at the end of every water step that touches
native.

## What we already know

- Native refuses the whole pipeline export on any value it doesn't know
  (`native/client/src/pipelines.rs`, `deny_unknown_fields`). It accepts only
  buffer, sampler and texture bindings, float/depth/uint samples and
  2d/2d-array views. So the first step that adds a storage texture, an
  `r32float` or `unfilterable-float` depth copy, or a new `sizes` key must
  teach native that value in the same step, or native stops starting.
- A new pipeline whose settings native knows is built and never used, which is
  harmless. Changing an existing pipeline's bind group layout panics at draw
  time (`by_layout`), and a changed sample count or target list fails
  validation at draw time.
- Native's tests (`native/client/tests/pipelines.rs`) build pipelines but never
  draw. "Native must run" needs a check that actually draws a water frame.
- Native renders parity plates by jumping between frames, so a wave field that
  carries state from frame to frame needs the same deterministic warm-up on
  both clients, or stays out of parity.
- The native gaps item "Judge water on a plate" is owned here now
  ([native gaps](../native-gaps/README.md)).

## Gaps

None yet.
