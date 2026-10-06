# Live map workbench

The local [workbench](../../../apps/map-workbench/README.md) tunes map construction and numerical validation policies on a live top-down plan. Drafts use the existing Rust generator, compiler, inspector and simulation analysis. Explicit saves replace reviewed fixture defaults. Ordinary Play selects an admitted battlefield automatically; developer seed inspection and replay remain exact.

## Why these boundaries exist

A developer can learn from a refused seed. A player pressing Play needs a battle. Those needs use different entry points: the workbench preserves each requested seed and its diagnostics, while [ordinary admission](../../../web/src/battle/prepare/admission.ts) tries bounded fresh candidates before an independently resolved released map. The admitted request becomes the address, labels and replay identity. On the same build and saved inputs, refreshing that address prepares the same battle without ordinary candidate selection.

The preview emphasizes causal geometry rather than production rendering. Clicking an apartment district identifies the rule for that district kind everywhere the preset is used; it does not create a hidden local override. Search opens the same fields. Detail inspection reuses the retained plan instead of generating another map with potentially different inputs. Feature selection belongs to that artifact: a new plan cannot silently reuse a district index as the old selection.

The saved comparison belongs to its saved sources and map choice. Editing the draft does not invalidate that comparison; changing the seed or adopting new saved sources does. Its label and input receipt continue to identify the saved geometry while the draft regenerates.

The browser presents Rust answers. A native developer executable gives the local tool access to plan inspection and sampled sight without adding a retained developer world or report API to production Wasm. The standalone and browser inspector share drawing, palette, caption formatting and crop capability; readable HTML notes are presentation of native text.

## What tuning may change

Construction policy, numerical quality requirements and finite search/work allowances are editable. Mathematical discretization, structural validity and physical proof assumptions keep their owning code. The [policy classification](parameter-policy.md) records why recorded constants became controls or remained safeguards; it is extraction provenance, not a replacement for current configuration.

A work allowance may refuse newly generated geometry earlier. It must not lower the supported admission of already released saved maps. Editable part/bay allowances fit within the fixed catalogue ceiling, preserving independently valid catalogue admission for developer fixtures and exact replays. The physical certificate cannot be disabled to obtain a preview.

Sampled openness is an assessment of eligible points, separate from continuous physical coverage, encounter readiness and a played battle. Its target can change without becoming a new generation rejection rule. Historical report/test callers retain their sampling choices through [sim::map_analysis](../../../crates/sim/src/map_analysis.rs).

Geometry measurements update with the plan. Sight remains on demand because additional browser-visible publication exceeded the user’s 500 ms condition on representative larger maps and denser sampling. A native inner-loop timing would omit world preparation and publication; enabling it automatically requires evidence for the complete extra work, without thinning the requested sample.

## Invariants

- Native report outcomes belong to exact captured source bytes and a type, size and canonical unsigned 64-bit seed. Source receipts, managed preset revision and existing physical generation hashes answer different questions. Analysis-only edits do not invent a physical identity. Sample execution failures explicitly mark their input evidence unavailable.
- Valid settings can be saved when a particular seed refuses. Review is read-only; Save publishes the server-held candidate after checking all source receipts and native validity. No-op saves preserve bytes. Stale or conflicting outside edits survive rejected publication and recovery.
- Both local editors coordinate source capture, publication and recovery through [FixturePublication](../../../apps/fixture-publication/publication.ts). Native work operates on captured bytes outside the storage queue. The shared owner prevents an in-process save from exposing rules before their derived catalog. A mechanics Save response resolves its catalog from the same captured bytes as its returned documents, including an outside edit after publication.
- One native job and one latest pending draft bound foreground work. Superseded output cannot become current. A refusal leaves the older admitted plan clearly labelled and inspectable; its measurements and sight results retain their own provenance.
- Displayed accepted/baseline artifacts are explicitly retained. A discarded successful job cannot evict either. Samples own temporary geometry and attempt source export before replacement; completed outcomes, unattempted seeds and missing receipts remain distinguishable.
- Saving invalidates future preset/default imports without refreshing a live battle. New navigation and explicit developer Restart load saved inputs. Running pages and captured replays retain their original inputs.
- Generated candidate selection has finite attempt/time limits and closes losers and timed-out workers. Permanent input/runtime faults do not rerun unchanged. Exhausted generation refuses the skirmish rather than substituting a fixed map.

## Owners and proof

The [native report](../../../crates/mapgen/examples/map_workbench_report.rs) composes existing generation and analysis owners. Its tests pin retained inspection, refused input receipts, empty sight, capacity containment and exact standalone/browser geometry. [WorkbenchStore](../../../apps/map-workbench/server.ts) owns source and artifact lifetimes; its [tests](../../../web/tests/mapWorkbenchServer.test.ts) exercise publication, cross-editor recovery, interleaving, stale inputs, cancellation and displayed-map retention. The [editor](../../../apps/map-workbench/src/MapWorkbench.tsx), its [behavioral tests](../../../web/tests/mapWorkbenchEditor.test.tsx) and the [runner tests](../../../web/tests/mapWorkbenchRunner.test.ts) own draft scheduling and repairable input.

The [real-native publication test](../../../web/tests/mapWorkbenchNative.test.ts) checks saved bytes and later reports in an isolated checkout fixture copy. Existing [generation parity](../../../web/tests/mapLayout.test.ts) and [preparation/replay tests](../../../web/tests/prepareBattle.test.ts) protect the unchanged exact path. The [workbench scene](../../../web/scenes/map-workbench.mjs) exercises the actual local route; the [generated scene](../../../web/scenes/generated.mjs) includes forced generation refusal and deadline cancellation through real workers. Root `check` and `verify` remain the complete closeout gates; no battle balance run is required because combat rules did not change.

The configured independent CLI review could not run because its model was unsupported by the account. Fresh independent app agents reviewed the feature’s code and final visual captures. This does not represent a successful CLI verdict.

## Visual provenance and rejected approaches

No uploaded picture defined the design. The established [Rust inspector](../../../crates/mapgen/src/inspect.rs) supplies the top-down geometry and palette standard. Whole-map framing, readable notes, detail legend wrapping, long measurement values, exact save panes and refusal provenance were checked on desktop and narrow captures. Runtime captures stay in ignored scratch output. The [exploration map](visualizations/decisions.html) preserves the user’s four-quadrant decisions; [choices](choices.md) records the final architectural discretion.

A game-rendered preview, named experiment library, local district overrides and combat/observer tuning were excluded by the user’s scope. Automatic sight without a complete cost proof was rejected. JavaScript geometry/LOS implementations, captions duplicated from Rust and “latest success” artifact retention were rejected because each creates another answer or loses the result actually on screen.
