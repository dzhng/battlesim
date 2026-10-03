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

## Outcome

**What was built.** The viewport now keeps two poses: the one asked for (input, a script, the opening framing) and the one drawn. `CameraController` still makes the first. A pure resolver in `renderer-core` makes the second, so that the eye and the near plane round it stay out of every building the side knows stands, and above the ground. GPU packing, picking, DOM projection and sound all read the drawn pose. There is no second rig.

**The seam.**
- `resolveClearance(state, desired, dt, obstacles, tuning, maxPitch) → Camera3DParams` (`packages/renderer-core/src/cameraClearance.ts`). It returns `desired` itself when that is clear and nothing is left to recover, else a new pose that looks at the same target from a clear eye. `state` (`ClearanceState`) is the caller's: the last desired pose, the eye last emitted, the easing, what holds the eye (`hold`: none, lift, slide, pushback), and the flags `cut`, `pressed`, `settled` and `blocked`. The first call, and a move of more than 32 m of eye travel in one frame, are placements.
- `CameraObstacles` (`cameraObstacles.ts`) is the obstacle view: `groundAt`, `ceiling`, `count`, `clear(point, clearance)`, `sweepClear(a, b, clearance)`, `sweepTop(a, b, clearance)` (how tall the tallest thing in the way is) and `pushOut(out, point, toward, clearance)`. `createCameraObstacles(boxes, groundAt)` builds it over a 32 m grid.
- `buildingObstacles(props, known, parts, groundAt)` (`packages/battle-renderer/src/buildingObstacles.ts`) supplies it at the viewport boundary, from the public map's building parts and the side's known props. It reads `knownStanding`, the list massing draws. `useBattleSession` rebuilds it only when what the side knows of a building changes, and hands it to `LabViewport` as the `obstacles` feed.
- `CameraController`: `CameraPresentation` gained `clearance` (validated in the constructor), and the class gained `resolve(state, desired, dt, obstacles)`. `step` and `place` are unchanged.
- `ViewportPilot.pose` may return null to leave the camera to the player for a frame (the lab rides and releases it); `attach` and `frame` became optional.
- Harness: `__lab.setCamera` is the raw framing, as before. New: `placeCamera(pose)` (the scripted path), `clearance()` (what holds the drawn pose), `flyClearance(poses, dt, reps)` (a scratch flight, for cost).

**The policy as built.** In order: the pose asked for when clear; the smallest clear lift up the orbit arc; the smallest clear slide round the target; pushback, which pushes the eye out of the box it is in through the side it is already on. Three things were added to make that smooth and are recorded in [`choices.md`](../choices.md#the-camera-may-wait-at-a-tall-building-then-cut-past-it): the policy looks 0.75 s ahead along the camera's own motion; a building in the way is gone over when its roof is within the lift's reach and cut past, with the cut reported, when it is not; and a goal keeps `release_m` more clearance than a drawn pose needs. The eye eases to its goal as a critically damped spring and is pushed out of whatever the easing would carry it into, so every pose emitted is clear.

**Tuning** (`fixtures/game.json`, `presentation.camera.clearance`). The lab's eight trajectories at 60 frames a second chose them. "Pressed" is how long the eye is held against a wall; "extra" is the most the drawn eye outruns the eye asked for.

| Number | Value | Alternatives measured |
|---|---|---|
| `margin_m` | 0.5 (1.82 m clearance with the 1.32 m near envelope) | 0.25 and 1.0 change nothing measurable; the benchmark tour passes 2.97 m over a village roof and is untouched up to 1.0 |
| `release_m` | 0.75 | 0.25: pressed 0.33 s at the wall and 0.57 s at the courtyard; 1.5: 0.20 s, extra 27 m/s |
| `lift_max_m`, `lift_steps` | 16, 4 | 12: the 18.5 m courtyard block and the compound are not lifted over (7 cuts, 13 s pressed); 24: no gain here, extra 28 m/s |
| `slide_max_m`, `slide_max_rad`, `slide_steps` | 16, 0.6, 4 | 8: 0.3 s longer pressed at the tower; 24 and 0.9 rad: 0.3 s shorter, still one cut |
| `refine_steps` | 6 | 3: the same |
| `smoothing_s` | 0.35 | 0.25: pressed 0.15 s at the wall, extra 36 m/s; 0.5: 0.47 s, 18 m/s; 0.2: 0.08 s, 47 m/s |
| `lookahead_s` | 0.75 | 0: 0.70 s at the wall; 0.5: 0.35 s; 0.9: 0.22 s; 1.0 and more lift the benchmark tour off its keyframes at 51 s |
| `sweep_step_m`, `sweep_max_steps` | 1, 32 | 2 m steps: the same |

**The lab's trajectories** (`fixtures/camera-lab.json`, flown by `/lab/camera`):

| Trajectory | Poses asking for an eye in a building | Held by | Cuts | Pressed | Extra |
|---|---|---|---|---|---|
| Wall: a dolly through an 18.5 m slab | 50 of 481 | lift | 0 | 0.27 s | 24 m/s |
| Tower: a pan whose eye passes through it (60.5 m) | 99 of 481 | slide, then pushback | 1 | 1.37 s | 10 m/s |
| Tower: an orbit that swings through it | 63 of 721 | slide, then pushback | 1 | 1.00 s | 15 m/s |
| Tower: a zoom straight down onto its roof | 216 of 481 | pushback (stops at the roof) | 0 | 1.40 s | 2 m/s |
| Corner: a pan into the elbow of two 30 m slabs | 210 of 481 | pushback (held in the elbow) | 0 | 1.53 s | 2 m/s |
| Courtyard: a dolly over a closed block | 117 of 601 | lift | 0 | 0.28 s | 25 m/s |
| Compound: an orbit at closest zoom in a U block | 396 of 721 | lift | 0 | 0.23 s | 17 m/s |
| Placement: cuts to framings inside each building | 367 of 541 | pushback, lift, slide | 5 (its own) | | |

No drawn eye came nearer a building than 1.82 m (the envelope is 1.32 m), at 60 or 20 frames a second.

**Cost.** The resolver costs 1.8 to 4.6 µs a frame flying the main town of Metro Large seed 1 (9,276 buildings, 13,579 boxes), testing 7 to 26 boxes a frame; the obstacle view is built once in 7 to 10 ms (development build, `CAMERA_MAP=metro:large:1` on the generated scene). On Mixed Small (3,452 boxes): 3 µs, 35 to 37 boxes, 3.4 ms. A unit test holds a query on a 10,000-box town under 20 box tests.

**Frame cost** (the village benchmark's short run, back to back on one machine, load average 20 to 38):

| | FPS | Frame p50 / p95 / p99 | GPU mean | CPU p50 / p95 |
|---|---|---|---|---|
| Before (`e5cf680`) | 47.3 | 24.1 / 25.7 / 33.3 ms | 6.29 ms | 1.5 / 2.7 ms |
| After, first run | 48.9 | 16.8 / 25.4 / 33.3 ms | 6.00 ms | 1.8 / 2.6 ms |
| After, second run | 48.1 | 17.4 / 25.7 / 33.3 ms | 6.29 ms | 1.4 / 2.3 ms |

No change beyond the machine's noise. The tour is drawn as flown: drift 0 between drawn and intended, and `cameraPaths.test.ts` holds that at 30 and 120 frames a second against the real village.

**Checks.**
- Unit (`cameraClearance.test.ts`, `cameraObstacles.test.ts`, `cameraPaths.test.ts`, 27 tests): no drawn frame or short move enters a box, by a measure apart from the resolver's index; placement inside a building; lift before slide before pushback, each bounded; a bounded number of questions a frame; long frames and fast input; reversal; a seeded rapid jitter; no flicker at a grazed roof line (and flicker without the release margin); idle recovery; the ground; unseen against seen falls. Eighteen deliberate breaks of the code each fail a test.
- Scenes: `camera` (10 checks, the lab), `generated` (two moves through the main town flown in real time by the viewport's camera, every drawn eye judged against every massing box), `village` (139 checks) and `benchmark` (10) unchanged and green.

**Before and after.** Eight matched pairs along the two moves through Metro Large's main town, the pose asked for drawn raw beside the pose drawn for it, with the battle paused and fog off. Before, the eye is inside a building, which is not drawn from inside, so the frame shows the town through its walls or a roof from beneath. After, the same target is seen from above the roof or beside the wall. Nothing else differs between the frames of a pair.

**What an unprimed critic saw** (19 images: the eight town frames, four film strips at 0.2 s a frame, seven lab frames; no backstory). No frame shows a building's inside, a see-through wall or a missing face. It did report:
- about a second of the wall dolly with the wall, then the roof, filling the whole frame, and the tower pass 70 to 100% tower face for eight frames. Both are the camera pressed against a wall; the wall dolly would look at that wall from any height, since its target is behind it;
- the jump in the tower pass between two frames: the reported cut;
- a roof entering and leaving the spiral's frame within three frames, covering up to half of it: the eye skimming a roof it was lifted over;
- uneven speed over the wall (a fast rise, then nearly still), and a yaw swerve in the town dolly as the eye is held off buildings while its target moves on;
- three lab frames with no building in them. They are correct (a camera pushed out on its target's side looks away from the tower; the fallen tower's remains are behind the camera) and poor evidence.

**Left open.**
- The camera waits at a tower's wall for a second or more before it cuts past, with the wall filling the view. The same happens in a corner of tall slabs for as long as the pose asked for stays inside them.
- At the closest zoom the eye skims roofs it has been lifted over at 1.8 to 2.6 m, so a roof can fill the lower half of the frame.
- No scene watches the camera while a real battle brings a building down; the knowledge boundary is tested at the list.
- Lab routes that build their own world without a battle session (geometry, foundation) get ground clearance only.
- The JS bundle's size was not measured before and after.
