# 03 — One `ScenePlayer`

## Contract unlocked

One player plays a capture scene for both the menu and the capture route. A shot can follow any unit of the encounter, either side. The battle score's shuffle comes from the scene seed.

## Seam

- **`ScenePlayer`** (React component) is extracted from `apps/battle-lab/src/MenuBackdrop.tsx`, where the pilot and the battle shell sit around lines 214-380. It owns:
  - the warm-up tick and elapsed time;
  - shot sampling and the follow subject;
  - the fade veil, the stage element and scene completion.

  Its props: the scene, cards on/off, and an `onEnd` callback. The menu keeps its grey film look and scene loop on top. Don't wrap it in another abstraction.
- **`reelPilot.ts`** is pure. It maps (reel, t, `unitAt(index)`) → camera pose. `unitAt` reads the unfogged (x-ray) observation, so red units resolve too. A followed unit that's gone holds its last position.
- **Score seed.** `packages/battle-audio/src/appAudio.ts:188` uses the scene seed instead of `Math.random`, in menu and capture alike.

## Human can see

The menu looks and cuts as before. A test capture with `follow` set to a red unit's index tracks it.

## Verification

- Pure tests in `web/tests/capture.test.ts` or beside the pilot:
  - pose at t;
  - following a red unit;
  - a vanished unit holds its last position;
  - the same seed gives the same score order.
- The menu first-cut probe and the `frame` scene.
- Visual: use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) to compare menu frames at the same reel times, before (slice 02) against after. Slice variable: camera pose and veil only. Then run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) as the last check.
- Human checkpoint, non-blocking:
  - open the pair with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md);
  - wait about 5 minutes;
  - if there's no reply, decide on the evidence, record it here, close the shots and continue.

## Delegated

File split and prop names, as long as there's one owner.

## Stays green

`menu_reel` (the battle is unchanged), catalog and capture tests.

## Out of scope

`graphicsTest.tsx` gets import changes only.
