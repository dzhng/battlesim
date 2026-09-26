# 19b — Seen-world darks: nothing seen reads as fog

**Status:** planned. **Depends on:** 15b, 18, 19. **Lane:** renderer.

## Contract

Slice 15b made the fog unmistakable: in every frame the critique called it "a blue-grey, diagonally hatched overlay with a thin white outline". But the standing gate question still got a yes on two frames, both caused by *seen* pixels that 15b is forbidden to touch:
- **`default-wall`:** the forest's flat, dark floor under the tree line reads as a slab with straight edges that could be shadow, ground tint or fog. The floor is slice 16's ground inside slice 19's woods.
- **`ground-hill`:** a near-black sun shadow whose caster is off-screen is darker than the fog and "can easily be read as the hidden or unexplored area". The shadow fill is slice 13's.

This slice fixes the seen world so that neither reads as fog, without post-processing seen pixels (the user's rule). The fix goes in the owning materials and light.

## API seam

- **Light (`presentation.light`):** raise the shadow floor. Tune the sky fill and ambient occlusion so no sun shadow on open ground is darker than the darkest `presentation.fog` preset's unseen ground. Also fix slice 13's reddish shadow on dirt if it touches the same numbers.
- **Terrain and trees (`fixtures/biomes/summer.json`, the terrain material, the scenery layer):**
  - break up the forest floor with leaf litter, roots and dappled light;
  - soften its straight rect edges into the field with a feathered verge, while the simulation's rect stays authoritative for rules;
  - keep it lighter than the fog.

  Slice 18's grass may carry some of this.
- No change to fog, and none to seen-pixel post-processing.

## Verification

- A measurable check in the `fog-look` scene: at the default and ground framings, the darkest 1% of seen ground is lighter than the darkest 1% of unseen ground under every preset, or differs by a stated hue margin.
- **Gate:** an unprimed screenshot-critique asked exactly "Could any dark region be mistaken for sun shadow, or any shadow for fog?" must answer no on all six 15b framings (`default-wedge`, `default-wall`, `default-shadow-edge`, `ground-street`, `ground-wall`, `ground-hill`).
- compare-screenshots against the WARNO woods and field crops.
- Record frame cost (benchmark short run).

## Decision budget

- **Delegated:** the numbers, within the fixture.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. The fog-vs-sweep agreement. Every existing scene and test. `bun run check` and `bun run verify` at closeout.
