# Infantry weapon use

A soldier works on one weapon at a time. Switching has no equip delay, but the selected gun still needs its ordinary aim and cycle. Vehicle mounts remain independent. An AT operator using his rifle visibly carries his launcher on his back.

The simulation owns selection; presentation observes it. Carrier assignment and active use are different facts: recovering a launcher does not permanently prevent rifle fire. Preserve existing casualty handoffs and stocked-spare assignments. Do not add a Stinger unit, target-based spare reassignment, manual weapon controls, equip timers, modular attachments, or balance tuning.

## Contracts

- Choose before any weapon advances or fires, including the first launch tick. Guidance has priority; otherwise use the assigned single weapon when it has a legal effective target, even while aiming/reloading. Fall back to a carried default gun when the single weapon cannot engage. If neither can engage, reload the assigned weapon when it needs work. Stable mount order breaks ties. Readiness must not starve a paused reload.
- Keep ownership and magazines across switches. Inactive physical cycles pause reload, clear aim and burst-start state, and allow passive cooldown to elapse. Aim belongs to the physical gun (`Cycle`), not the shared squad targeting lock. Other riflemen retain their own progress. Existing stationary-movement interruption still applies.
- Publish each visible member's active mount with their stable identity. Keep the mount's `operator` as carrier assignment. Enemy member filtering applies to active use as it does to identity; no hidden live-state reads. Include authoritative selection and aim in the digest.
- A mount's optional `operator_appearance` becomes `{ active: [...], carried: [...] }`. This is a hard cutover; no compatibility parser. The active set is launcher-held; the carried set is rifle-held/launcher-on-back. Both use the same variant identity and their own clip families. Ordinary soldiers keep their existing appearance. A dead carrier never duplicates the recovered launcher.
- Model, animation, facing, and muzzle attachment resolve the same active/carried state. An equipment-family change resets incompatible pose blending. No renderer inference from reload timers or squad-wide historical shots.

## Slice graph and review surface

1. [Simulation](slices/01-simulation.md): exclusivity and independent physical aim.
2. [Observation and posing](slices/02-observation.md): depends on 1; publish selection and consume it.
3. [Carried launcher art](slices/03-carried-art.md): generation can run alongside 1; integration depends on 2.

Independent planning drafts examined smallest shipping ladder, ordering/deadlock risks, and asset/seam ownership. Their common conclusion was to retain physical weapon identity and add authoritative activity, rather than deleting rifle cycles or changing carrier assignment to impersonate switching.

Visual checks compare matched native-size captures and feature crops against the pre-change battle and the requested equipment relationship. Run compare-screenshots, then an unprimed screenshot-critique as the last visual acceptance check, and show the result with preview-shots. Scratch captures and logs belong in `throwaway/infantry-weapon-use/` per AGENTS.md. Existing art, lighting, and terrain readability remain outside this equipment change.

## Next Agent Prompt

Finish browser closeout and archive. Core selection, publication, poses and carried art are reviewed and committed. Full native/web/browser/balance runs have completed; reuse their unaffected green results. Fresh movement tests, Clippy, typecheck, native/Wasm publication and generated-map parity pass. The generated-town test now retains its original assertions and timeout while using the math owner's signed distance directly.

The balance controller spreads rejoining squads using its existing per-unit destination helper. Matched before/after runs complete all 50 trials without refused orders. The conservative clear fine-route optimization preserves every final trial row and digest exactly, and the existing search bound now passes. It changes work, not routes or battle outcomes.

Browser repairs are being checked narrowly: admitted village/panel/muzzle/contact staging, shipped projectile fixture ranges, the authored road material row, and controlled light/building comparisons. Keep all original physical and visual requirements; do not retune gameplay to satisfy staging. Saved logs, paired balance results and falsification probes live in ignored `throwaway/infantry-weapon-use/`. Finish the integrated review and choices consolidation after remaining checks pass. Then archive the rationale and accepted visual provenance. Gameplay uses ordinary deaths; the unused carried-model death endpoint remains excluded.

- [x] Close mixed-target garrison regression and simulation review.
- [x] Publication schema, native privacy and packed identity codec tests.
- [x] Fresh Wasm/native publication and generated-map parity after the final native change.
- [x] Held/carried model, clip, facing and mount-aware muzzle selection.
- [x] Generated carried variants and bounded bake; unrelated runtime identities preserved.
- [x] Matched visual comparison, fresh critique and Preview checkpoint.
- [ ] Full closeout checks, paired balance evidence, integrated review/choices consolidation and archive.
