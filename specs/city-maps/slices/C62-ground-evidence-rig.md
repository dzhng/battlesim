# C62: ground evidence rig

**Depends on:** C60. **Kind:** slice.

## Question
Can every ground slice shoot one frozen station, get an exact mask of the ground class it judges, and get the same pixels twice?

## Contract it unlocks
- **Stations:** `web/scenes/_groundStations.mjs` holds named poses per map id. For the village:
  - `bend-25/65/120/250` at the (420,420) road corner;
  - `forest-edge-65` and `forest-deep-25` in the west wood;
  - `field-65/250` and `patchwork-1100`.

  Lab maps add theirs.
- **A `FrameView` `ground-classes`,** written by the terrain material's own shading function (not hand-drawn). Per pixel it encodes road signed-distance bands, river bands, forest (inside, verge or none), plot kind and a hashed plot id.
- A `suppressTrees` toggle beside `suppressModels` (trees are scenery).
- The frozen set: fixture light, `setClock(T0)`, biome seed, no units, scars and paint suppressed, DPR 1 at 1920×1080.
- Baseline shots of every station on main before any ground change. These are each sub-lane's first "before".

## API seam
`packages/battle-renderer/src/scene.ts` (`FrameView`), `apps/battle-lab/src/LabViewport.tsx`, `web/scenes/_groundStations.mjs`.

## What the human can run or see
A station sheet: each shot beside its class mask.

## Verification
- Two runs give byte-identical masks and final shots within tolerance.
- The mask agrees with the sim export at 1,000 sampled pixels.
- The mask view adds 0 ms to the final view.

## Delegated to the implementer
Mask encoding; stations beyond the village minimum. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
The final view's pixels; every scene.

## Feedback that would change this slice
A camera station that hides the judged variable changes the evidence rig, preserving fixed comparison inputs.
