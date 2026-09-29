# Audit cleanup

Repair the implementation review at `25c273a708087e5b9149638c82b83ff2d28eb387`. The user's end state is net less code and complexity, with equal or better performance. If a repair requires unnecessary machinery, stop at that issue and ask whether its product/mechanics contract should change; do not silently narrow scope.

## Next Agent Prompt

Continue this whole goal, not just the current batch. The source audit and reproduction artifacts are in `throwaway/implementation-review/`; all ten headline findings and all detailed repair/conditional-combine records are in scope. Revalidate against current code. Use write-tests red/green, refactor-clean, and review. No future city-map/HUD feature work is authorized here. Baseline: clean `25c273a`; 416 Rust + 466 web tests pass; 26 browser fixtures finish with one unresolved pause assertion failure. Preserve every independently valuable test contract when consolidating.

Current work: parent owns input selection and integration; isolated lanes own simulation, rendering, and browser lifecycle. Update this handoff and the evidence ledger as work lands. Do not run shared GPU gates concurrently. Full check/verify run once on integrated changes; targeted runners are the feedback loop.

## Contracts and batches

- [ ] Input: one living selection for HUD and commands; click recognition owns its own selection transitions. Repair Stop browser proof and selection integration.
- [ ] Simulation: a complete remembered prop pose; garrison admission/planning uses side knowledge until discovery; squad cover area uses authored capacity, not corpse history. New authoritative state enters digest.
- [ ] Lifecycle: preserve terminal direct-transport replies; pause reflects user intent across scheduler stalls; dispose late viewport builds before publishing readiness/listeners.
- [ ] Rendering/assets: latest appearance request owns GPU commit and impostors; one bake writer; shared GPU schema owner; remove proven obsolete helpers.
- [ ] Test repairs: every Repair in Rust inventory and web/package/harness reports resolved with effective proof or corrected claim; conditional combines retain all unique assertions first.
- [ ] Cleanup: diagnostic recording has explicit consumers; simplify overloaded ownership where supported, without mechanical file splitting or compatibility shims.
- [ ] Verification: narrow red/green evidence; simulation instruction counts and outcome comparisons; fresh WASM; full check and all browser scenes; visual changes receive compare-screenshots, preview-shots and unprimed screenshot-critique.
- [ ] Closeout: whole-diff review and independent Codex review; final issue-by-issue audit; production/comments/tests/docs sizes show net reduction, performance evidence shows same or better.

## Evidence

Baseline review logs and benchmark report are preserved in `throwaway/implementation-review/`. Detailed live evidence and issue accounting belong in `progress.md`; decisions belong in `choices.md`. No fix is complete merely because a test is green.
