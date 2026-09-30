# Choices

Three independent drafts agreed on one target owner per mount and a separate cycle per physical weapon. Cadence and magazines ship together because splitting them would replace the same state twice. Finite ammo now means actual rounds, not squad volleys. Stable soldier IDs prevent casualty/replacement state leakage; a special weapon retains its cycle on handoff.

Initial magazine values preserve the previous rapid shot intervals and introduce clear reload pauses. These are game values, not a claim of real weapon specifications. Suppression widens dispersion rather than reducing damage after a hit. The existing shooter-tagged launch feed already owns individual flashes and recoil.

Baseline quick report: flank captured 3/3, blue cost lost 538 total; ambush captured 0/3, loss 0. Saved under throwaway/fire-cadence/before.json.

The final quick comparison captures 2/3 flank runs within 600 seconds versus 3/3 before. Blue cost lost changes from 538 to 630, with no flank tanks lost. Ambush captures remain 0/3; one seed now loses a tank (200 cost versus zero before). These rule changes intentionally alter battle outcomes; no damage or unrelated accuracy values were retuned to compensate.

Wider suppressed scatter exposed an existing own-cover inconsistency: props were protected from their shooter's rounds, but a vehicle hull used as cover was not. Flight now carries the same cover identity for either body. The existing lean regression went red and passes with that invariant restored.

Native matched tracer captures measure a median bright width of 3→1 px at tactical zoom and 6→2 px close up. The first thin candidate used a subpixel core footprint; the final version keeps a larger raster footprint while reducing world width and the glow floor. Fresh review accepts visibility in both the isolated pair and 24 crowded gameplay frames. Fine diagonal aliasing remains apparent when enlarged; native trails read continuously. Sampling at 0.1 seconds does not prove absence of every between-frame flicker.

Review covered ownership, diff, and documentation. The external Codex CLI review could not run because its configured model was rejected by the account service; it is not counted as a passing review. Renderer-skill instructions were validated with a fresh small-model scenario and pushed to the shared skills repository.

The reference image is the user's September 29, 21:15 gameplay capture, cropped only to remove browser chrome. It documents the reported excessive rifle thickness and volley appearance, not an approved visual target. It is reference-only and never loaded by the game.

A browser priority assertion assumed the enemy tank remained alive at tick 470. A direct reproduction showed it dead with no identified enemies, so an area target was correct. The scene now requires and checks the actual period when the live identified tank and a firing report coexist, instead of pinning a stochastic death time.

The full closeout report completed 50 trials at 900 seconds. Flank captures were 10/10, with no flank tanks lost; the quick 600-second cutoff had left one seed still running. This is a current-state report, not a paired full baseline. Raw reports and visual evidence are in `throwaway/fire-cadence/`, and the filmstrip is reproducible with `WATCH_TOURS=cadence bun run --cwd web scene -- village-watch`.

The general battlefield capture no longer requires a vehicle casualty. It still requires live fighting and craters, and frames a wreck when one exists. Dedicated destruction and wreck-effects scenes retain their unconditional wreck checks. The complete browser run found these two casualty-timing assumptions; both affected scenes passed their focused reruns after correction. All other scenes passed the complete run. Full code checks passed (364 Rust tests and 472 web tests); the benchmark measured 66.7 FPS average, recorded in the battle-look frame-cost ledger.

The final fresh reviewer inspected the recaptured 24-frame sequence and matched before/after pairs, accepted the thinner rifle treatment, and found no blocking ghosting or layering defect. It noted minor diagonal pixel stepping/brightness variation at native scale. The gameplay framing is nearer than the historical report and omits its lower recon squad; it is a relevant firing-cluster view, not an exact recreation of the entire screenshot. Soldier-specific muzzle flashes are visible; exact timing remains the simulation tests' claim rather than an inference from stills.
