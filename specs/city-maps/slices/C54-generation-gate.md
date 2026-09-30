# C54: integrated map generation and all-map gate

**Depends on:** C53, C55, C34–C36, C56, C05, C09, C60, C31, C50, C59, C16–C19 and C32; C57 for playable camera evidence. **Kind:** required integration gate.

## Question
Do offline/runtime generation and every catalogued map satisfy the accepted composition, physical access and full-extent budgets beyond one attractive seed?

## Contract it unlocks
`mapgen validate <preset-set>` inspects compiled MapDefinition with shared sim geometry queries and reports exact extent, metre scale, town hierarchy/count/area, category/floor distribution, usable plains, roads/bridges, entrance access, forest area/count and top/bottom coverage differences. It includes physical infantry/vehicle routes and range/approach metrics independently of coverage fairness. C59's prepared encounters supply legal deployments, objectives, defender inputs and garrison references across the matrix.

The generation/lowering owners already enforce their construction contracts; this gate verifies their integrated physical outputs and records evidence. It does not introduce another generator, duplicate geometry oracle or a battle format. Catalogue maps and runtime requests resolve through C09. Saved maps pin SOURCES.json; runtime results/replays pin equivalent generator/preset/physical-catalogue/map identity; appearance hash is separate. C56's migration inventory is part of this gate; pre-existing maps are not exempt.

## API seam
`mapgen::validation/report` over compiled geometry → CLI/workbench gallery and release verdict. C60 describes saved catalogue entries; transient generated seeds need no persistent catalogue folder. C51 owns encounter composition.

## What the human can run or see
A gallery covering all nine type × size combinations, with at least ten fixed seeds per combination, plus an inventory of migrated village/lab/benchmark/test maps and their preserved arena checks. Show failed seeds with diagnostics; no substituted layouts.

## Verification
- Full canonical native/wasm map/encounter/rules/identity/diagnostic parity for matched inputs, including boundary seeds and failure cases.
- Every category/family selectable by final presets has accepted release art for required states. Explicit prototype rows cannot satisfy release; remove the temporary massing source when coverage is complete. G0 owns the supported family matrix.
- Exact selected extents, stable building/street scale, intended Open/Mixed/Metro character, central Metro hierarchy and class exclusions.
- G0's per-type counts/shares/weights and comparable top/bottom town/forest metrics without requiring mirrored geography.
- Connected physical roads/bridges, usable plain approaches for 1,800 m weapons, entrance access and both infantry/vehicle transitions.
- Every catalogued map has real urban/plain geometry; C56 arena contracts remain intact and map/digest changes are named.
- Full-extent generation/startup/peak memory, steady-state and snapshot publication, default/strategic/pan/zoom frame budgets. Benchmarks run serially; no generation inside battle ticks/frames.
- Compare whole-map and tactical composition against accepted per-variable evidence using compare-screenshots. Run unprimed screenshot-critique last on accepted shots; open the gallery with preview-shots non-blocking and record the verdict.

## Delegated to the implementer
Fixed seed selection and report/gallery presentation. Numeric thresholds must already be ratified at G0.

## Must stay green
All-map scope, fixed dimensions, one generation/compiler/resolver path and replay identity.

## Feedback that would change this slice
A failed type/size or arena contract reopens its owning slice; one showcase seed cannot waive the gate.
