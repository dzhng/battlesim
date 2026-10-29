# Water on native (placeholder)

**Status:** placeholder. Native and the browser draw the water through the same
[shared renderer](../../crates/renderer-core/README.md), wakes included; one gap
is listed: the wave field on parity plates (see [Gaps](#gaps)).
**Updated:** 2026-10-29.

## Next Agent Prompt

You are picking up what is left of
[water and wetlands](../done/water-and-wetlands/README.md) on the native client
([`native/client`](../../native/client/README.md)). That spec's
[decision 19](../done/water-and-wetlands/decisions.md#scope-and-acceptance)
asked native to draw water at least as well as the browser; the shared
renderer now does that by construction. Walk the gap below with the user, then
run [`write-spec`](../../.agents/skills/write-spec/SKILL.md) on what they want.
Load [`renderer`](../../.agents/skills/renderer/SKILL.md) before any GPU work.

## Gaps

- **Wakes on parity plates.** The wave field carries state from frame to
  frame (`renderer-core` [`water.rs`](../../crates/renderer-core/src/water.rs)),
  and its clock runs at most `WAVE.max_steps` steps a drawn frame. Native
  renders parity plates by framing the records between plates without drawing
  them, so its field has run fewer steps than the browser's at the same plate.
  The [water scene](../../web/scenes/water.mjs) compares the mean water colour,
  which wakes barely move. Plates that judge wakes need the same deterministic
  warm-up on both clients, or must stay out of plate parity.
