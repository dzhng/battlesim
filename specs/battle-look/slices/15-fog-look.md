# 15 — Fog look and contact ghosts

**Status:** planned. **Depends on:** 14. **Lane:** renderer.

## Contract

How fog looks (F1):
- **Seen:** the real world, unprocessed.
- **Unseen:** dimmed and cooled toward night, readable, and never mistakable for sun shadow. Live-tunable, because the user will test other looks later.
- **Last-known contacts:** a hatched ghost outline plus the existing red glow.

## API seam

- `FogStyle` in `presentation.fog {dim, cool, saturation, edge_softness}`. It is identity on seen pixels. It applies to every world material, including translucent surfaces and impostors.
- `ContactGlyph` draws a hatch plus the red glow after post, from existing approximate contacts only: no class, exact position or motion.
- `/lab/fog-look` with sliders that write a copyable fixture block.

## What you can run or see

`/lab/fog-look` and `/battle/village`.

## Verification

- Tests:
  - seen pixels are identical with fog on and off;
  - every material path takes the style;
  - contact glyph expiry;
  - no inferred data on contacts.
- The critique runs at a 16:00 sun, with long shadows beside fog edges.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Visual variable

- **Variable:** separation of unseen from seen and from sun shadow.
- **Reference crop:**

- `armaphract/x-urban-fog-t9s.jpg` and `x-urban-fog-t7s.jpg`: the unseen areas' readability.
- The hatched contact above the intersection in t7s.

- **Out of scope:** The sight boundary's position (slice 14) and the lavender halftone. The seen area must stay unprocessed.

### Visual acceptance (in this order)
1. Capture the named frames through the slice's scene, at fixed seed, tick, camera, 1920×1080 and DPR 1. Evidence goes in `throwaway/evidence/<fixture-id>/`.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the candidate crop against the reference crop above. Record the telemetry and a less-wrong verdict for **this variable only**. It is not a pixel match.
3. As the **last check before acceptance**, run an **unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)**. Give it only the frames and a neutral task. Include the question: *"Could any dark region be mistaken for sun shadow, or any shadow for fog?"* Act on its findings, or record why not.
4. Open the frames with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) as a **non-blocking** checkpoint. Allow about 5 minutes and keep working meanwhile. If there is no answer, decide on the evidence, record the decision in the verdict, and close Preview.


## Open look questions from spike 02

These are resolved here, with a non-blocking user checkpoint:
- **Roofs and canopies above eye level.** True per-pixel sight leaves every roof and tree canopy above a soldier's eye unseen, so they read as fogged even inside a fully seen village. Provisional call: a surface counts as seen when its column's ground is seen, for surfaces above eye height within the ground cell's footprint. Record the rule you ship.
- **Fog beside sun shadow.** At a 16:00 sun, sight shadows cast from buildings run beside those buildings' sun shadows, and the spike's critique found dim-and-cool alone did not keep them apart. This slice must add a cue that sun shadow never has, from the look board: a hue shift, a boundary line, or texture. It must pass the standing question: "Could any dark region be mistaken for sun shadow, or any shadow for fog?"

## Decision budget

- **Delegated:** Default dim, cool and hatch spacing, within the fixture.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

The user may later try other unseen styles: colour highlight or more processing. That changes `FogStyle` only.
