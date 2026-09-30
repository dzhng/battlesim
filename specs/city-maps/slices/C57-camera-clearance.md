# C57: hybrid camera clearance

**Depends on:** C01 public building geometry, C22 template rendering and G0's resource limits. **Kind:** required slice.

## Question
Can zoom, pan, orbit and scripted placement remain outside buildings with nearby pose recovery and smooth pushback?

## Contract it unlocks
Keep CameraController as the intent owner. Add one pure clearance resolver over desired/previous orbit poses, elapsed time and an indexed public/side-known obstacle view. It tests eye clearance and swept motion, tries a bounded nearby lift/slide set, then smoothly pushes to a safe pose; recovery also stays safe between samples. Initial placement, rapid input, idle recovery and benchmark/controller `place()` use the same policy. No parallel rig is introduced.

Clearance margins include the near-plane envelope needed to avoid visible wall penetration. Named tuning data bounds candidate count, lift/slide displacement, smoothing and hysteresis. Measure and record those reversible values with trajectory evidence; an unsafe interpolated pose is never emitted to make motion look smoother.

Use public authored building/terrain geometry plus changes learned by the viewing side. Hidden live destruction cannot change camera behavior. Wire this view through the viewport/static-world boundary; renderer-core has no sim-state dependency. A harness-only raw framing API, if necessary, is explicitly named and never used by playable/benchmark camera paths.

## API seam
`renderer-core` camera intent/projection plus pure clearance resolver; public/side-known obstacle adapter at the viewport/static-world boundary. Both CPU picking and GPU projection consume the resolved pose.

## What the human can run or see
A camera-clearance lab with scripted zoom/pan/orbit/placement trajectories around walls, towers, corners, courtyards and concave compounds. Show desired/resolved eye paths and recovery behavior.

## Verification
- Eye/near-envelope stays clear throughout swept/interpolated movement, including large dt, initial placement and direction reversal.
- Nearby pose attempts precede pushback; bounded search and hysteresis avoid oscillation. Idle recovery works without new input.
- Hidden versus observed building destruction respects knowledge; clipping geometry and displayed art do not diverge.
- Indexed obstacle query cost and frame-cost row within G0's budget at Large Metro density; no full-building scan per input/frame.
- Camera motion is the only visual variable. Compare matched trajectories before/after with compare-screenshots, run unprimed screenshot-critique last, and preview-shots non-blocking.
- Apply game-ui, renderer and math skills before implementation; use the existing math/projection owner.

## Delegated to the implementer
Internal query/index plumbing and bounded numerical tuning after measured trajectory trials, recorded with alternatives. Policy order, no-penetration and knowledge boundary are fixed.

## Must stay green
One projection/intent owner, safe motion and no hidden-state leaks.

## Feedback that would change this slice
Jarring recovery changes bounded tuning or candidate order within the accepted nearby-pose-then-pushback policy.
