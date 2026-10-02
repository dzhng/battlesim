# C54: integrated map generation and all-map gate

**Depends on:** C53, C55, C05, C09, C60, C31, C50, C59, C16–C19 and C32; C57 for playable camera evidence. **Kind:** required integration gate.

## Question
Do offline/runtime generation and every catalogued map satisfy the accepted composition, physical access and full-extent budgets beyond one attractive seed?

## Contract it unlocks
`mapgen validate <preset-set>` inspects compiled MapDefinition with shared sim geometry queries and reports exact extent, metre scale, town hierarchy/count/area, category/floor distribution, usable plains, roads/bridges, entrance access, forest area/count and top/bottom coverage differences. It includes physical infantry/vehicle routes and range/approach metrics independently of coverage fairness. C59's prepared encounters supply legal deployments, objectives, defender inputs and garrison references across the matrix.

The generation/lowering owners already enforce their construction contracts; this gate verifies their integrated physical outputs and records evidence. It does not introduce another generator, duplicate geometry oracle or a battle format. Catalogue maps and runtime requests resolve through C09. Saved maps pin SOURCES.json; runtime results/replays pin equivalent generator/preset/physical-catalogue/map identity; appearance hash is separate. C56's cutover inventory is part of this gate; pre-existing maps are not exempt.

## API seam
`mapgen::validation/report` over compiled geometry → CLI/workbench gallery and release verdict. C60 describes saved catalogue entries; transient generated seeds need no persistent catalogue folder. C51 owns encounter composition.

## What the human can run or see
A gallery covering all nine type × size combinations, with at least ten fixed seeds per combination, plus an inventory of cut-over village/lab/benchmark/test maps and their preserved arena checks. Show failed seeds with diagnostics; no substituted layouts.

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

## Outcome: pipeline tool checkpoint

The native `battle_sweep` example now follows the game's generation request,
physical compiler, assault planner and Scenario/Battle owners. It retains one
JSONL outcome for every exact type, size and fixed seed, with canonical input,
generation, recipe and rules identities. A refusal keeps all diagnostics; an
unwinding per-case panic keeps its message and does not erase later cases.
This is failure isolation, not recovery from a hung process or memory exhaustion.

The attacker advances its planned column by an ordinary group move; each unit's
acknowledged offset destination is what the report measures. Existing defender
scripts and policy remain in the scenario. Every unit remains visible in the
report, including uncommanded and dead units. Proximity within 10 m, distance
travelled, remaining distance and time in each movement state are distinct
fields. `goals_not_neared` names lack of proximity during the sample, not a
failed arrival or proof of permanent blockage. Dead ticks cannot count as moving
or planning. Generation, prepared geometry, placement, Battle build and tick
costs are separate; progress and JSON formatting lie outside the tick bracket.
Instructions are null where unavailable. Native counters and timing reads add
measurement overhead; loaded wall time is not performance-change proof.

The refusal matrix and acknowledged-goal/dead-state report tracers pass; assigning
one unit's goal to the group fails the latter. A real Open Small seed-1 assault
ran for 30 s, with every column unit setting off, no refused placement and no
substituted input. Its distant routes remain pending within the short window.
The complete matrix, native/Wasm integration, physical/access/gallery and release
art gates remain open. This is the tooling half delegated to the scale lane.

### Current physical-catalogue matrix

The source `13beacf9` run retains all 90 requests (Open/Mixed/Metro ×
Small/Medium/Large × seeds 1–10), layout-7, catalogue `6b0a5e8b…`, rules
`a59680c1…` and engine `6b3eb26d…`. Generation succeeds throughout. The
planner refuses Mixed Medium seed 9: none of three tried settlements balances
the two jeep approaches within 15 s after its allowed 1,500 m adjustment.
The refusal remains a parent planner finding; no seed is substituted.

The other 89 battles play 80,100 ticks. Of 1,335 units, 1,334 survive and 979
acknowledged goals are not neared within 30 s; most are kilometres away and in
transit. Ten infantry units remain planning in Open Medium seeds 5/8 and Metro
Small seed 10, requiring focused extension/diagnosis. The three previously
frozen vehicle failures terminate with explicit obstruction in their separate
120 s replay. This closes neither the new infantry finding nor encounter
playability. Loaded wall peaks (378 ms; 489 ticks over 33 ms) are diagnostics,
not the 100-a-side throughput admission or instruction-gain proof.

### Historical layout-6 matrix control

The first complete tooling run keeps all 90 type × size × seed requests
(seeds 1–10), historical `layout-6` / `layout-presets-6`, physical catalogue
`5416684f…`, rules `a59680c1…` and engine `0f4153f7…`. Generation succeeded on
every request; Mixed Medium seed 9 had the same approach-balance refusal recorded
above. No refused seed was replaced.

The other 89 requests each play 900 ticks: 80,100 ticks total, ten loaded-wall
ticks over 33 ms, peak 72.51 ms. These are small assault rosters, not the
100-a-side admission arm. Of 1,335 units, 1,334 survive; 979 recorded goals are
not neared in the short run. Most are kilometres away and still in transit.
Mixed Small seeds 3/9 leave their supply unit stationary, and Metro Medium
seed 9 its tank; all three remain planning through a focused 120 s extension.
Private attribution locates repeated failed final road connectors: the first
case rejects only 43 of 1,034 goal accesses in 30 s, paying tens of thousands
of expanded cells for each. Its reverse query exhausts the destination pocket
in 36 cells. The subsequent strict final-connector correction and frozen-case
rerun are
recorded in [C06](C06-sim-scale-passes.md#outcome--shared-final-connector-component).
This historical sweep does not establish current simulation admission.
