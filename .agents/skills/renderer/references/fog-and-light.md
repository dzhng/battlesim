# Fog and light

The sight fog is the look's signature, and **fog must never read as sun shadow, nor shadow as fog**. Most of the spec's critique rounds were spent on that one rule. Light and fog are judged together: a fix to either is checked against the other.

Owners:
- `frame/fogTerm.ts`: the per-fragment term;
- `frame/fogVisibility.ts`: the horizon-map builds and tile cull;
- `frame/fogMaskPass.ts`: the screen-space edge, compose and rim;
- `frame/fogInputs.ts` and `frame/fogStyle.ts`: the fixture's geometry and styles;
- `frame/environmentFrame.ts` and `light/`: sky, sun, cascades and post.

History is in `specs/done/battle-look/spikes/02.md` (landmines) and that spec's `choices.md` (the fog-look entries).

## How the fog works

- **Sight lights:** a polar horizon map per own eye.
  - Each map has 4096 azimuth bins by 64 log-spaced radial bins.
  - Each bin packs an f16 horizon slope, a u8 position where the horizon jumps inside the bin, and a u8 foliage length.
  - Terrain is marched coarse (512 rays); props are intersected analytically at full azimuth. That took a build from 8.5 ms to 2.4 ms.
- **A map rebuilds only when its eye moves.** Turret traverse and the sight shape are applied per fragment and never trigger a rebuild. `rebuild_eyes_per_frame` caps the work.
- **A tile cull reads the prepass depth.** It writes per-16 px-tile eye lists (`tile_eyes_max`), and it lifts its depth bounds by the tallest canopy, because canopy and water aren't in the prepass.
- **`FogTerm(worldPos, normal, pixel, isGround)`** is called by every world material. A material never shades fog itself.
- **Each world fragment writes two targets,** `WORLD_OUT {color, fog}`:
  - the lit HDR colour;
  - a coverage mask `(unseen, seen, ground, alpha)`. Units and sky write zero.
  
  The MSAA resolve turns samples into coverage fractions.
- **The mask pass runs a separable bounded distance transform:** rows, then columns, capped at `FOG_DISTANCE_CAP_PX`.
  - Compose applies the style to unseen pixels in HDR, before post.
  - The rim draws after post in display colour, so its colour is exact and never blooms.
- **Effects draw after the world, single-sampled.** They lower the mask by their strength, so bursts show over fog.
- **Agreement with the simulation is tested by oracles.** Rust's sight vectors are pinned against the TS and WGSL mirrors, and a CPU lookup oracle mirrors f16 rounding. Street agreement is about 0.6% of 8 m cells against a 5% bar. Deliberate WGSL mutations must turn these checks red; if a change of yours doesn't, the check is broken.

- **Fog extends past the playable area; the border marks it.** The backdrop past the map edge and the scenery on it call `FogTerm` like the map's ground and trees. The horizon maps already run on past the edge as open ground: no occluders or foliage, the running horizon held. So units see past the edge, and there is no seam or "outside is unseen" case. The playable area is marked instead by a red border (`playAreaOverlay.ts`, `presentation.map_border`, an overlay: never fogged, width in pixels by zoom step). This is presentation only: the simulation's knowledge stops at the map edge. An early "everything off-map is unseen" rule was rejected by the user, because it cut the sight shapes off at the edge.
- **Overlays test against the prepass depth, copied before the grass draws** (`FrameTargets.overlayDepth`). Tested against the colour pass's depth, every grass blade punched a hole in route ribbons and rings, which read as speckle. The same prepass is split so the own units' x-ray (the unit-in-woods cue) draws against the world without units, with a depth bias of about 1/128 of the distance, so bodies touching the ground aren't flecked.

## Landmines already paid for

- **Probe placement.** Probing above a surface lights roofs from afar, and probing inside it self-occludes. Probe 0.1 m out along the normal, and count only eyes in front of the face.
- **Radial lag lit a strip of every roof.** Store where in the bin the horizon jumps, and switch there.
- **Grazing facades sawtooth.** Interpolate the jump distance across the two nearest rays, then choose once.
- **Foliage.** Interpolate linearly inside the bin; stepping it gave 7% disagreement in forests. Foliage saturates at 8 bits, so the full block must fit in that range.
- **The rules differ by kind:**
  - trees take fog whole at the crown heart, with no facing test (leaf normals split crowns into stripes);
  - grass probes the ground under each clump's root;
  - roofs use "air in front" (`roof_reach_m`);
  - corpses take the fog of the ground at their feet;
  - units are never fogged.
  
  `models/modelFog.ts` binds the group per model class.
- **Rim only across ground.** Rimming every seen/unseen boundary drew white rings round trunks and roofs.
- **Unseen means more than half the samples are unseen ground and none is seen.** "More than half" alone put rim dashes along every wall silhouette.
- **Softness can't live in the material.** A fragment knows no neighbours, which is why the mask pass exists.
- **Bloom leaks fogged HDR onto nearby units,** so "seen pixels are identical with fog on and off" is checked with bloom off.
- **Open gap: no self-occlusion.** A thin up-facing ledge on a prop's hidden side (sandbag courses, lintels) probes into seen air and reads partly seen.
  - An instance box, low props as occluders, and screen-space cleanup were all rejected.
  - A real fix needs a baked fog bent normal or a per-model hull, which belongs with the model pass.
  - The `fog-look` style check counts at a 2 px margin because of it.
- **Cleared forest cells re-upload only the foliage,** but every new cleared cell rebuilds every eye's map. Watch that when lanes are carved in view.

## Never reads as shadow: what worked, what didn't

- **Dim-and-cool alone fails.** In the spike and again in slice 15, a sight wedge read as a second blue cast shadow, and a sun shadow inside fog as a second fog tone.
- **What works: cues sun shadow never has.** Screen-anchored hatch lines (5 px pitch, 1.5 px wide, 45°) with a line floor, a pale rim on the seen side of the edge, and a soft edge.
- **Seen pixels are never touched by fog.** If a seen region reads as fog, fix the light or the terrain material instead. `light.shadow_floor` (0.4) keeps sun-shadowed surfaces at 40% of the sun, in its hue, so the darkest seen ground stays lighter or apart in hue from the darkest unseen ground under every preset. A scene check holds this.
- **Don't copy WARNO's near-black wood shade.** Under `dusk` it reads as fog.
- **Many things read as shadow or as fog in critiques. Avoid these patterns:**
  - two-sided noise blobs on terrain (they read as cloud shadow; ship one-sided dry strips at constant luminance instead);
  - a darker, wider road verge;
  - a dark wet-soil shore;
  - a dark collar round a crater, or a bowl floor darker than 0.7;
  - a scorch wash;
  - a normal tilt past 40° (it gives a grey sky sheen that reads as fog).
- **Ask the critique directly:** "Could any dark region be mistaken for sun shadow, or any shadow for fog?" It's the standing last question for any look change.
- **Standing critique findings left as known:** hatch moiré on walls, the rim outlining grass like frost, distant fog strips reading as water, and fogged wreck faces reading as paint. Don't rediscover them as new.

## Light and shadow

- **Cascades split over the part of the map the camera sees** (a 13×13 screen-ray grid against the map box), not from the near plane.
  - Settings: 4 × 2048², capped at 2,600 m.
  - Receiver depth must use the same normalisation as the breaks, or several cascades shade one receiver in dark bands.
- **Penumbra is a world width** (`softness_m`), turned into a per-cascade PCF radius clamped to 1–4 texels. The normal bias scales with that radius, with a 3 cm floor.
  - A wider width with a fixed bias self-shadowed lit ground as screen-door noise.
  - A fixed 0.6 m bias detached soldiers' shadows.
- **Give each cascade its own caster camera buffer.** A shared buffer lets every pass see the last write queued.
- **A cold frame publishes a legal empty shadow block,** so the receiver never samples uninitialised memory.
- **Fill and sky.** Teal shadows came from sky-only fill under a blue sky. The fix is a warm fill standing in for bounce light, not a warm grade: a grade tint recolours every dark albedo and the fog. `sky.radiance` is one knob for the view and the environment together.
- **Map edge:** a large backdrop with the ground material and the biome's distant palette, plus below-horizon sky sampling and haze. Without it the map reads as a plate in a void.
- **Known defects left alone:** 1/255 banding on flat ground, stippled 5-tap PCF with no temporal filter, and building umbra harder than thin casters' shadows.
