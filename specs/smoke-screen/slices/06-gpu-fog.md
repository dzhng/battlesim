# 06: GPU fog and the smoke lab

## Contract

The fog you see on screen closes behind a cloud and opens again after its fade.
It follows the same stepped law and depth as the simulation. A developer lab
route lets anyone pop smoke and watch it from both sides.

## Seam

- **Presentation.** Slice 02's `set_smoke` is fed each frame from
  `ObservationFrame.smoke_clouds`, through `contract::smoke::state` at the
  publication tick. The source is `presentation/side_fog.rs`, the same path
  eyes take. Promote slice 02's prototype into its owning module and delete any
  probe-only scaffolding.
- **Lab route.** Add a `smoke` lab route in `apps/battle-lab/src/routes/`, with a
  fixture registered in `apps/battle-lab/src/fixtures.json` and a small `test`
  map:
  - two `test_smoke_tank`s and a `test_smoke_squad` against an enemy tank and a
    guided-missile team, matching the [mock](../visualizations/mock.html)
    layout;
  - a view switch between the two sides;
  - a scripted pop, until slice 08 adds the button.

  Mark it as a test page, per the browser-checks rules in `web/README.md`.

## What you can see

The smoke lab route. Pop smoke and switch sides: the enemy's fog closes over
the carrier, and its last-contact marker stays put.

## Tests and checks

- Fog-sight vectors with a cloud: the eyes near it rebuild, and the eyes away
  from it don't.
- A frame-cost check in the lab at 100 eyes, against slice 02's table.
- **Visual checks:**
  - load `renderer` and `game-ui` first;
  - take shots before and after the pop, from both sides;
  - judge them with `compare-screenshots` against the mock's view;
  - run an unprimed `screenshot-critique` as the **last** check;
  - show the user with `preview-shots`. This is non-blocking: allow about 5
    minutes; if the user is silent, decide on the evidence, record it in
    `choices.md`, close the shots and move on.

**This slice's one visual variable:** where the fog is lit and dark around
clouds. Drawn smoke (07) and UI (08) are out of scope, and stand-in visuals are
fine.

## Delegated

You may choose the lab's layout details, the camera framing, and how the
scripted pop is triggered.

## Must stay green

- The presentation and renderer tests.
- Every existing lab route.
- The frame-cost tripwire.

## Feedback that would change this slice

If the fog edge reads as wrong on screen (too hard, or too soft), adjust the
fog's display softening, not D1's rule.
