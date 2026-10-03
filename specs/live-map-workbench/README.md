# Live map workbench

A local developer tool for tuning generation and numerical map-validation policy. Draft inputs run through the existing Rust generator, compiler, inspector and simulation analysis. The browser presents a plan and measurements, and explicit saves publish the exact reviewed fixture changes. Ordinary Play independently selects an admitted battle instead of handing a generator refusal to the player.

## Next Agent Prompt

Status: planned, 2026-10-03. Implement the entire ladder below, not just its first pass. Start with slice 01 and the source-policy inventory; implement independent player recovery concurrently. The next dependency is the native report/publication contract, then the plan editor and sampled analysis. Read each slice before work. Use write-tests red/green at behavioral seams, review and audit-choices before each focused commit, and update this prompt at every checkpoint. Do not declare automatic sight enabled until its additional browser-visible cost is measured below 500 ms. Full check and verify run once after all slices finish. No backwards-compatibility or migration machinery is requested; exact existing seed links and captured replay are live contracts, not scaffolding.

- [ ] [01: inputs, source-policy ownership and capacity](slices/01-policy.md)
- [ ] [02: native report and source publication](slices/02-report.md)
- [ ] [03: live plan editor](slices/03-editor.md)
- [ ] [04: sampled analysis and seed sample](slices/04-analysis.md)
- [ ] [05: ordinary Play recovery](slices/05-play.md)
- [ ] [06: composed verification and closeout](slices/06-closeout.md)

## Contracts and scope

The accepted walk is retained in [the decision map](visualizations/decisions.html). It records the user choices and delegated reasons. Generation controls include settlement/district mixes, roads, parcels, rivers, country furnishing, forest placement and props. Validator controls include numerical fairness/approach/transit/coverage policies and finite work allowances. Physical forest/observer rules, combat settings, encounter editing, hand-editing placed geometry, new terrain relief, arbitrary playable dimensions and production rendered preview are excluded.

The preview is a top-down plan. Search and map selection lead to the same fields; a selected apartment district identifies its generating kind, not a local block override. Saved and draft comparisons hold type, size and seed fixed. Valid settings may be saved even if the chosen seed refuses. Every requested seed in a sample retains its outcome; map admission, encounter readiness and sampled sight remain separate claims.

The additional sampled-sight cost budget is strictly below 500 ms, including world preparation, native invocation, analysis, serialization and browser publication after an accepted artifact exists. Until measured, ship on-demand sight. Do not lower sampling quality to satisfy this budget. Keep existing caller-specific sight spacings in historical tools/tests.

The added player requirement is ordinary Play → admitted battle. Keep exact preparation pure: one explicit source and encounter. Put bounded fresh-seed selection above the existing worker client, then a released saved fallback. Success publishes the actual admitted request/address. Explicit seeded links, catalogue links and replay remain exact. Generation work budgets do not lower released-map admission.

## One owner per concept

- Presets and their Rust schema own construction/admission/work policy. Classify all recorded constants in [parameter policy](parameter-policy.md), promote true policies into their existing preset groups and preserve exact default values. Numerical/physical correctness stays in its existing owners.
- `generated-battle.json` owns generation limits and a typed analysis-policy subsection. Analysis policy is assessment, not geometry; its source fingerprint changes without inventing a new physical identity.
- `mapgen` owns one generation/lowering pipeline and plan inspection. A native developer example can use its existing sim dev-dependency; no production mapgen→sim dependency and no developer-only production Wasm API.
- `sim::map_analysis` owns the sampled sight calculation extracted from the shared example. Reports/tests/tool call it; no JavaScript ray casts.
- The local Vite API owns source snapshots, native process/artifact lifetime and publication. Browser requests never name file paths. Reuse/extract fixture-publication primitives from mechanics without inheriting its unit store.
- Source revision, tested-input fingerprint, preset revision and map/config hashes are distinct. Preserve exact tested documents with exported reports. Manage revision labels in the tool; the user does not edit them.
- Native generation returns/retains plan and compiled map from one invocation. Inspecting a crop, selecting a feature or panning never regenerates unchanged settings. Retain at most the saved baseline, latest draft and current job; clean artifacts when their session ends.
- One active generation and one replaceable latest pending draft. Background sample work yields to an edit through cancellation, keeping completed outcomes and marking unfinished requests. Leaving terminates owned processes/workers, not merely their callbacks.

## Review and verification

Each slice owns its fastest behavioral gate and a runnable route/probe. Initial extraction preserves the existing default plan/map bytes, refused outcomes and exact-path battle digest/replay. An intentional new ordinary candidate is a new identity, not parity drift. Mapgen Native/Wasm paired records remain the oracle; do not re-bless default geometry to accommodate extraction.

For each visual slice, compare-screenshots judges before/after and the native inspector reference; screenshot-critique is the last visual acceptance check using a fresh unprimed agent; preview-shots shows the user a compact set. Human review is non-blocking for reversible presentation. Use normal developer tool surfaces for the workbench; apply game-ui to player loading/menu/recovery. Evidence and probes stay in ignored throwaway. Source references and decision artifacts belong here; runtime captures do not.

A final full check and verify happen once at slice 06. Workbench/report/preparation policy requires no played balance report because no battle rule changes. Do not count a compiled route or passing DOM test as visual verification. Full gates retain inherited failures rather than weakening requirements.

## Slice graph and delegated decisions

01 → 02 → 03 → 04 → 06; 05 can start independently and joins 06. Report and local store can be separate workers against the typed protocol, while player recovery is isolated. Each worker uses its own checkout and shares installed dependencies, never build output.

Internal names, small reversible layout choices, debounce within 250–400 ms, native report serialization and concise field help phrasing are delegated. Policy classification must record its source/call-site rationale; no unclassified value becomes a knob by accident. Concrete recovery budgets and sight enablement are measured decisions owned by 05/04. Prefer two generated candidates with an eight-second total selection allowance initially; reduce based on fallback headroom if needed. Never extend the existing under-30-second startup requirement.

## Research and draft synthesis

Three independent drafts used compact, risk-first and seam-quality lenses. All preferred native local tooling over a new browser report API. The vendor consultation (Claude Opus) was attempted read-only and bounded without usable output; it is not counted as an independent successful verdict. Actual sources and existing report/inspector behavior ground the plan. No new external library or unfamiliar rendering technique is required.
