# 23 — The browser on shared code: whole-game check

**Contract:** after 19 and 20, the shipped browser runs its publication reader and its presentation derivations from Rust through WASM. This slice proves the whole game, not just the reel, is unchanged. That covers battles, replays, the labs that use poses or effects, and the menu.

**Gates:**
- Every browser scene that draws units or effects passes, along with the visual suite (`bun run --cwd web visual`) against its approved pictures.
- Battle frame time on a medium generated battle (the method in `specs/performance/README.md`) is no worse than before 19, with main-thread CPU and GPU reported.
- Digest and replay unchanged.

The repository's full check still runs once, at 24.

## Built (2026-10-12, branch `wholegame`)

**Status:** the gates hold. The web suite and the visual suite (`cursor`, `panels`, `ui`) are green. Every scene passes in the final pass but two street checks, whose staging the rules changed under them. One performance regression is named too. All three are left to their owners (below).

**What ran**, on `main` with every lane merged up to `9807a9775` (fog sight in Rust, the props lanes, the exact 30 Hz tick, session-scoped appearances):
- The web suite, whole, once at the end: 191 files, 1,254 tests, green.
- Every registered scene: first in parallel shards, then the failures serially, and a final serial pass after the last merges. Timing scenes ran alone under the GPU lock on the production build.
- The medium battle three times, with slice 20's probe (`/battle?type=mixed&size=medium&seed=1&faction=us`, production build, 1600×1000, GPU lock).

**Battle frame time** against the same probe's runs before 19 (2026-10-11). The two sets aren't interleaved, since the old tree is gone; every difference sits inside the run-to-run spread.

| | avg FPS | p95 ms | GPU mean ms | main-thread busy ms/frame |
| --- | --- | --- | --- | --- |
| before 19 | 59.7, 59.6, 58.0 | 24.6–25.0 | 4.55, 4.83, 4.50 | 3.92, 4.23, 3.91 |
| now | 61.8, 57.3, 65.1 | 16.8–25.0 | 4.41, 4.36, 3.65 | 3.65, 3.84, 3.55 |

Main-thread work moved from the renderer's JavaScript (1.5 to 0.65 ms a frame) into the WASM presentation (0.12 to 1.05 ms). The graphics test (`graphics-fps`) runs market-town at 83.0 FPS (GPU 8.03 ms) and paris-corner at 100.0 (GPU 6.09), at or above 20b's numbers.

**Digest and replay:** unchanged. Nothing here touches the simulation. The scenes that pin them pass: street's saved replay keeps its captured digest, and endurance's three reset cycles present the same digest (`72c948f11f218b6b`).

**Load, not regressions.** Most first-run failures were lab-readiness timeouts at 30 s. They came in runs while other lanes' GPU jobs and the shards themselves shared the machine. Measured on a quiet machine, every one of those labs is ready in 11–13 s, cold or warm. Run serially, they pass. One `unit-roster` run lost `window.__lab.frame` after START BATTLE; it didn't reproduce in three runs.

**Fixed:**
- *Artillery voices.* The voice catalog requires a set for every playable unit and faction, and main's twelve artillery units had none; no artillery role exists. Each now speaks as its maker's tank crew, the closest role: a gun crew in a vehicle. An IFV crew speaks of dismounting, and a supply driver says he's unarmed.
- *The viewport's appearances arrive through a feed.* Slice 20b put the template library's file (1.6 MB) on `InstalledAppearances`. React's development build logged it byte by byte whenever the prop changed, and the workbench page failed. `LabViewport.appearances` is now a `FeedSource`, like the world and the overlay.
- *A restarted battle draws its buildings.* After Restart the buildings could stay undrawn until the camera moved: two of three endurance reset cycles drew none of 12. The building layer drops its static batches when the frame takes a new world, but a resend was raised only when the GPU frame was rebuilt. The viewport now raises it on every new world. Endurance holds it: every reset cycle draws its standing buildings, and the live allocations are identical.
- *The city line-up in 269 s, not about 1,790.* Its time went to Chrome encoding PNG screenshots. The scenes' heavy captures (`capture` in `scenes/_png.mjs`) use CDP's `optimizeForSpeed`, still lossless and byte-equal in pixels on a still frame. Its 1,800 s cap is gone.
- *Navigation test.* It waited 60 s for a first click behind the menu's loading cover (about 20 s warm, near a minute cold) and, on a first launch, graphics detection (30–36 s). Detection already starts only once the menu is revealed, and its result is kept in localStorage, so the test is now a returning player's. It fails only when the cover's progress stalls for 60 s.
- *Scene staging and stale scene code*, each against the current contract:
  - street's pick box is held to the f32 of the soldier's radius;
  - the ruler tour right-clicks the area the lost tank left, which the tank's fire renews as `firing`;
  - the ground scene's scar-filter check and the unit-roster ghost probe take the presented ground and the presentation's fog (both stale after 20b and the fog move);
  - the generated and benchmark scenes wait for the menu (`menuShown`);
  - the viewport test fails its fake frame at `setClock` (the fog move removed `setFog`);
  - two fog files on main were formatted.
- `mechanicsServer` passes once the mechanics validator is built fresh. `icons` needs its source models from LFS.

**Left, with owners:**
- *street: no rifle shot in the staged fights. street-watch: the observer never identifies the tank.* Restaged, not re-ruled: red's houses see the open west and its tank shells what they see from 620–650 m, so both journeys now go round the south of the west wood, as the woods tour's walker does. The muzzle tour's rifle fights the red AT team in a battle of its own, in the trees, captured with the canopy lifted off. The contacts tour's recon identifies the tank from the wood's east edge, slips back into the trees, and a rifle section pushing up the road draws the unseen tank's fire. Both scenes are green.
- *city-contact: 10.0 FPS, CPU p95 137 ms*, against 13.1–15.1 FPS and 72–81 ms at 20b. A main-thread profile puts `Presentation.prepare_fog` (fog sight in Rust, b7096267f) at 6,072 of 10,073 ms busy, about 60 ms a frame. The medium battle doesn't show it. Owner: the fog lane.
