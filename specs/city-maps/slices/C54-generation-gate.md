# C54: integrated map generation and all-map gate

**Depends on:** C53, C55, C05, C09, C60, C31, C50, C59, C16–C19 and C32; C57 for playable camera evidence. **Kind:** required integration gate.

## Question
Do offline/runtime player maps satisfy the accepted composition, physical access and full-extent budgets beyond one attractive seed, while catalogued developer arenas retain their original contracts under M29?

## Contract it unlocks
The existing generation, compiler, simulation queries and report tools supply one retained evidence matrix. `generate_map` and its compile report own normal generation admission, diagnostics and identity; layout measurement owns plan composition; the simulation's prepared map, encounter planner and battle reports own legal deployment, physical access and executed movement. The gate records exact extent, metre scale, hierarchy/count/area, category/floor distribution, usable plains, roads/bridges, entrance access, forest coverage and top/bottom differences, without introducing a second geometry oracle. C59 supplies recipe placement and legal deployments/objectives/defender inputs/garrison references; C51 owns final playable acceptance.

Keep each report's scope explicit. Predicted road times and pre-country layout metrics do not prove executed routes or final foliage. Generous compiler limits do not prove admission under game limits. Individual standardized sight probes do not prove the renderer's combined published opening fog. A planner route or an accepted order does not prove live arrival.

The generation/lowering owners already enforce their construction contracts; this gate verifies their integrated physical outputs and records evidence. Catalogue maps and runtime requests resolve through C09. Saved maps pin SOURCES.json; runtime results/replays pin equivalent generator/preset/physical-catalogue/map identity; appearance hash is separate. M29 cuts C56's added surroundings for developer arenas; their resolver, identity and original behavior checks remain required.

## API seam
Existing generation/compiler/simulation/report APIs → retained gallery and release verdict. There is no separate `mapgen validate` CLI or `mapgen::validation/report` module. Each measuring tool's code and owning README define its command and output schema. C60 describes saved catalogue entries; transient generated seeds need no persistent catalogue folder. C51 owns encounter composition.

## What the human can run or see
A gallery covering all nine type × size combinations, with at least ten fixed seeds per combination, plus catalogue identity/loading and preserved developer-arena checks. Show failed seeds with diagnostics; no substituted layouts.

## Verification
- Full canonical native/wasm map/encounter/rules/identity/diagnostic parity for matched inputs, including boundary seeds and failure cases.
- Every category/family selectable by final presets has accepted release art for required states. Explicit prototype rows cannot satisfy release; remove the temporary massing source when coverage is complete. G0 owns the supported family matrix.
- Exact selected extents, stable building/street scale, intended Open/Mixed/Metro character, central Metro hierarchy and class exclusions.
- G0's per-type counts/shares/weights and comparable top/bottom town/forest metrics without requiring mirrored geography.
- Connected physical roads/bridges, usable plain approaches for 1,800 m weapons, entrance access and both infantry/vehicle transitions.
- Furnished open country against the [user's countryside references](../assets/reference/broken-arrow/SOURCES.md#open-country-reference-target): short tree belts, copses, sparse rural homes and small physical objects between the towns, with most ground remaining open. Retain M25's sampled simulation sight result separately from the visual verdict. Capture the actual starting column's combined published fog at the normal opening camera: individual-eye samples do not prove the union's boundary has visible interruptions. Include a tactical field view and an overview; preserve the main settlement's 1,800 m approaches.
- Global coverage under M24/M25: every playable location on every generated map has interesting physical surroundings and some interrupted ground-level circular sight. Include actual circular observer ranges/eye heights, intervening positions between grid centres, edges and unbuilt town areas. Construction must account for failed placements rather than silently accepting uncovered land. A standard 600 m rifle, finite sample grid or successful opening view cannot waive a 450 m jeep's empty circle or prove continuous/all-seed coverage. Report narrower directional observers separately without promising an obstruction for every heading inside required clear approaches.
- Player maps have the required town/plain geometry and rendered surroundings. Catalogue identity/loading and original developer-arena contracts remain intact; map/digest changes are named.
- Full-extent generation/startup/peak memory, steady-state and snapshot publication, default/strategic/pan/zoom frame budgets. Benchmarks run serially; no generation inside battle ticks/frames.
- Compare whole-map and tactical composition against accepted per-variable evidence using compare-screenshots. Run unprimed screenshot-critique last on accepted shots; open the gallery with preview-shots non-blocking and record the verdict.

## Delegated to the implementer
Fixed seed selection and report/gallery presentation. Numeric thresholds must already be ratified at G0.

## Must stay green
Player-map matrix and developer-arena checks, fixed dimensions, one generation/compiler/resolver path and replay identity.

## Feedback that would change this slice
A failed type/size or arena contract reopens its owning slice; one showcase seed cannot waive the gate.

### Candidate opening checkpoint (2026-10-02)

The [open-country review](../assets/open-country-review/README.md#actual-opening-view-checkpoint) records actual production fog for two layout-12 candidate seeds, with full frames, masks and crops reviewed unprimed. Both overviews show genuine forest cuts and the oblique views show scattered tree lines/copses. Close-field variety, rural-house readability and overview label contrast remain open. The same review records a physically legal 450 m jeep circle that the standardized rifle report missed. This checkpoint does not admit global coverage, the final matrix or the landscape look.

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

### Historical layout-7 physical-catalogue matrix

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
