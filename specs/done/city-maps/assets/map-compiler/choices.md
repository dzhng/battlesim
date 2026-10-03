# Compiler preparation choices

All entries are sound for the bounded Pass 1 contract. Full compiler shape admission,
physical source/appearance fit and complete battle capacity remain open dependencies.

## Content identity and execution policy have separate meanings

**Choice:** Extract the existing fixed JSON/SHA256 owner into `contract::identity`.
The physical catalogue keeps its exact canonical bytes and hashes. Generation identity
holds caller-supplied version labels, a canonical decimal-u64 seed, plan hash, physical
catalogue hash and final map hash. Execution limits/report and appearance are excluded
from geometry identity. Authored sequence order remains meaningful; the compiler does
not sort away implicit-ID assignment or promise equivalence under arbitrary row reorder.
Version identifiers must contain a non-whitespace name; this prevents an empty provenance
field without inventing a release-version grammar or verifying its caller-supplied claim.

**Gap:** C04 required shared identity without choosing a numeric seed boundary, a new
hash implementation or canonical configuration ordering. Version labels alone cannot
verify actual source provenance; preparation acquisition must establish that separately.

**Reach:** Native, WASM and saved-map metadata use one record. Plan feature IDs identify
diagnostics/configuration, while the final map references physical template/part IDs.
Changing only admission limits leaves the map and identity exact. No preset or generator
randomness is introduced by this hand-authored core.

**Verdict:** Sound. **Confidence:** high for exact content identity; caller source/version
verification remains pending C09.

## Caller admission counts physical output before materialization

**Choice:** Require explicit maximum authored parts and cumulative bay positions.
The existing template owner counts its admitted lattice, and the existing authored-ID
owner resolves explicit reservations and remaining ordinary IDs. There is no compiler
ID allocator or second lattice formula. Count limits are checked before geometry cloning
and bay-vector materialization. Playable bounds follow the shared architecture envelope;
rendered surroundings and release presets are separate owners.

**Gap:** A per-descriptor bay allowance does not bound repetition across many placements.
Bounds and body-count limits also cannot establish whole-world startup/memory admission.

**Reach:** Ordinary bodies plus selected building parts count toward admission; runtime
trees/bridges, terrain/nav storage, catalogue/input JSON residency and complete battle
cost do not. Preparation callers must calibrate their policy against the admitted load.
Named feature IDs must be nonempty/unique so a failure refers to one authored placement.

**Verdict:** Sound. **Confidence:** high for bounded materialization, deliberately limited
for whole-world resources.

## Physical JSON precision belongs to the existing primitive owners

**Choice:** Share the existing raw-token numeric reader across templates, ordinary
`PropDefinition` geometry and map-header scalars. The admitted value must survive saving
and reload without changing its hash. Flattened authored/event envelopes retain raw
tokens until the existing geometry/action owner reads them; the flattened JSON schemas
stay the same. Finite physical numbers are required. No global serde float switch or
simulation trigonometry change is included.
Derived full box height must also remain finite when the base is implicit. The original
admission checked doubled height only with an explicit base and accepted an infinite top.
This representability rule adds no source dimension or production-model limit.

**Gap:** The actual native compiler accepted a supplied coordinate, but the old parser
changed its f64 value by one ULP. Saving map headers exposed the same failure. Serde's
flattened content buffer then discarded tokens before the exact reader could see them.

**Reach:** Precise newly admitted physical JSON now retains its supplied value, so an
input that previously rounded differently has an intentional numeric interpretation
change. Original complete physical/observation/digest receipts remain frozen. Canonical
box bounds reuse pinned template math for preparation admission only; S6's separate
simulation arithmetic issue is not claimed resolved.

**Verdict:** Sound. **Confidence:** high for measured save/reload and preserved original
contracts, limited to the physical fields that opt in.

## One preparation outcome crosses native and WASM boundaries

**Choice:** The library returns either a complete map/identity/report or diagnostics,
tagged `ok`/`error`. The CLI and WASM share that owner. CLI admission refusal exits 1;
usage/filesystem failures exit 2 and report their actual error. Successful file preparation
writes final `map.json` and identity `SOURCES.json`; this is not C60 catalogue publication
or an atomic update of an already acquired catalogue entry.

**Gap:** Separate wrappers could silently discard unsupported fields, use different seed
or numeric parsing, or make saved output differ from runtime generation.

**Reach:** Exact complete CLI/WASM records cover accepted and refused requests. No map
is returned or written on an admission refusal. Acquisition must verify identity before
using/publishing saved files; a filesystem error is reported, not converted to success.

**Verdict:** Sound. **Confidence:** high for preparation parity; publication/acquisition
remains owned by C09/C60.

## Partial compiler capability is explicit

**Choice:** Use the actual C03 `SurfaceArea` schema for plan surfaces and accept an empty
layer. Nonempty surfaces require shared contract admission before lowering. Regions,
rivers, forests and other requested unowned fields produce stable unsupported diagnostics;
none are silently dropped. There is no compiler polygon validator, parallel river/forest
schema, catalogue-vs-appearance coupling or generated-map runtime branch.

**Gap:** The shared shape owner has not yet moved out of simulation. Reimplementing its
validator would create another physical interpretation before C65/C69/C72 establish theirs.

**Reach:** The C03 schema/hash disposition is recorded separately from original records.
This pass proves building materialization/identity, not overlap/layout composition, source
readiness, art fit, the inspect overlay or the full C04/G0 gate.

**Verdict:** Sound. **Confidence:** high for the independently testable seam, with explicit
dependencies for the deferred arms.
