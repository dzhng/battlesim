# 12 · The Armory camera

Map row: L7. Depends on 11.

## Contract

The Armory has its own orbit and zoom camera on `CameraController`, with an
Armory config:
- left-drag orbits;
- the wheel zooms within the unit's bounds;
- no edge pan and no WASD;
- it takes input from the stage canvas only, never from `window`;
- pitch may go below the battle's floor;
- the eye clears the unit's hull and the yard props.

It is not `LabViewport`.

**After this slice, the Armory is shippable to players** on the placeholder
yard.

## Seam

- **The hook.** `apps/battle-lab/src/armory/armoryCamera.ts` exports
  `useArmoryCamera(canvas, bounds: Bounds, spot: Spot): PresentedCamera`.
- **The config is data.** It goes in `fixtures/presentation.json`
  `camera_armory`, read the way `gameCamera.ts:11-14` reads `camera`.
- **Pitch.** The Armory's own pitch range replaces `PITCH_LIMITS`
  (`cameraController.ts:82`), as a config field. Battle cameras keep theirs.
- **Clearance** goes through renderer-core's clearance (`ClearanceTuning`). Its
  `groundAt` includes the yard's prop boxes and the unit's bounds.

## The human runs or sees

On `/armory`:
- drag and wheel with a mouse;
- drag with a Mac trackpad, which has no middle button.

## Verification

A camera tour in `web/scenes/armory.mjs`. For one unit per spot, drag and wheel
to every extreme: 8 yaws × 3 pitches × the minimum and maximum zoom. The probes
check that the eye:
- is never inside the unit's bounds or a prop box;
- is never below the ground;
- doesn't move on keys, on edge hover, or on drags that start on the list or
  the sheet.

**Visual gate.**
- **The one variable:** framing at the extremes.
  - The closest zoom fills the frame without clipping the hull.
  - The lowest pitch shows the underside against the sky.
- **Crop:** the full stage at four extremes, with the panels masked.
- `screenshot-critique` last.
- Then the non-blocking `preview-shots` checkpoint.

## Delegated to the implementer

- Speeds, curves and inertia: config data.

## Must stay green

- Every user of `gameCamera`.
- `_cameraClearance.mjs`.
- The battle camera's pitch limits.

## Feedback that would change it

The feel is tuned through config data.
