# Research

External practice this spec leans on, with the call each source informs. Reference images, and why each was chosen, live in [`assets/reference/`](assets/reference/), with one `SOURCES.md` per game. The settled interview is [`unknowns-map.html`](unknowns-map.html).

## Code we port: `~/dev/game` (read-only, copied by manifest)

A production TypeGPU battle renderer for a medieval game. The pieces to port:
- the view-fitted shadow map, with cascades on High;
- terrain material;
- the GPU-skinned crowd with four mesh LODs and impostors;
- grass and far grass;
- scenery;
- the physical sky and PMREM;
- bloom plus AgX grade;
- the appearance-bundle asset pipeline (`packages/soldier-assets`: GLB tiers, one skeleton, clips, bounds, an impostor far pose, Blender bake scripts).

Its renderer choice is backed by a 32-tour held benchmark, recorded in `specs/done/battle-performance/`. TypeGPU and raw WebGPU beat three.js on 1% lows. The reuse traps (camera uniform layout, terrain sampling, no fog, HDR colour, ownership) are landmine cards 9–15 in the unknowns map.

## Fog of war with sharp line of sight

- The technique: treat each friendly unit as a light and render its depth ("sight map"); a pixel is seen if any unit's sight map sees it. [3D fog of war using multipass shadow mapping (GameDev.net)](https://www.gamedev.net/forums/topic/644054-3d-fog-of-war-using-multipass-shadow-mapping/)
- The cheap variant: a per-source angular depth map (a 1D "max blocking slope per angle" map), sampled per pixel. This is the likely fit for 100 units a side on a heightfield plus box props. [IrredenEngine: line-of-sight occlusion in the reveal field and the CPU oracle](https://github.com/jakildev/IrredenEngine/issues/3662)
- A precomputed field-of-view survey, for comparison: [FOV Mapping (ObjectOrientedLife)](https://objectorientedlife.github.io/personalprojects/FOVMapping1/).
- The genre precedent for buildings, trees and wrecks blocking sight: Company of Heroes 2, noted in the same threads.

The visual target is ARMAPHRACT: sharp building "sight shadows", an unprocessed seen area, and unseen dimmed. Fog must never read as sun shadow.

## Dense grass

- [Procedural Grass in Ghost of Tsushima (GDC 2021 Advanced Graphics Summit)](https://gdcvault.com/play/1027033/Advanced-Graphics-Summit-Procedural-Grass):
  - blades generated on the GPU as Bézier curves, each with its own appearance;
  - one 2D noise wind field that both the CPU and GPU sample;
  - staged culling: distance, frustum, occlusion.

  [A readable summary.](https://tigerabrodi.blog/what-we-can-learn-from-grass-in-ghost-of-tsushima-renders)
- `~/dev/game`'s `grass.ts` and `grassField.ts` are the starting point: residency plus far grass. The talk informs extending it to WARNO's dense, swaying look.

## Soldier rig and animation (CC0 candidates)

- [Quaternius Universal Animation Library](https://quaternius.com/packs/universalanimationlibrary.html): 120+ clips on one humanoid rig, CC0, with a `.blend` source. It covers locomotion, crawling, gun and death clips. [Universal Animation Library 2](https://quaternius.com/packs/universalanimationlibrary2.html) adds 130+ more.
- [Quaternius Universal Base Characters](https://quaternius.com/packs/universalbasecharacters.html): CC0 base bodies on the same rig, the likely soldier body under our own Blender kit.
- `~/dev/game/packages/soldier-assets/assets/incoming/PROVENANCE.md` records the Quaternius library as "not yet accepted", and it rejected Mixamo as not redistributable. The asset spike must confirm licences before anything is copied.
- Technique for vehicles and kit: [Muster WWII (MIT)](https://github.com/Kenton-GMI/muster-ww2). It uses analytic parts, bevels and edge wear, plus a headless multi-view review harness. Its soldiers are static 112k–176k-triangle sculptures, so they are not usable animated.

## Smoke, fire and explosions

- [Unity free VFX flipbooks (CC0)](https://unity.com/blog/engine-platform/free-vfx-image-sequences-flipbooks): smoke, fire and explosion sequences in EXR/TGA.
- [CGHEVEN CC0 flipbooks and VDBs](https://cgheven.com/blog/top-free-smoke-fire-vfx-assets-for-real-time-rendered-use).
- [Kenney Smoke Particles (CC0)](https://kenney.nl/assets/smoke-particles).
- The looks to match are Broken Arrow's filmic fire and volumetric-looking smoke, and WARNO's big bright fireballs and dust walls: soft, depth-faded particles lit by the sun.
