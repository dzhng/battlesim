# Aggregate physical ownership evidence

C01 moved garrison houses from `map.props` into `map.buildings`, each building an aggregate of physical parts with one owner. The original production observations and battle traces are preservation oracles, never refreshed from the aggregate implementation. New test inputs author the same physical boxes through the C00 preparation seam, keeping their original PropIds, poses, fitted appearance and seats. Only deliberate identity columns and the separate C02 input-field relocation are normalized when comparing. The oracles (`fixtures/parity/buildings/`) left the tree with the cutover proved; tag `data-files-before-prune-2026-10-01` holds them.

## What is proven

The [native outer tests](../../../../crates/sim/tests/buildings.rs) cover one integrity across asymmetric parts, strongest once-per-blast damage, terminal replacement chains, exposed seats and sensing eyes, hidden knowledge, clicked-part admission, bounded IDs, and the immutable-versus-active owner distinction. The [browser outer test](../../../../web/tests/buildings.test.ts) uses production Wasm, public map geometry and the real picker before decoding atomic replacements.

The animation stream still matches every original animation, header and fog word, after exactly the new owner limb split and KnownProp identity additions. Ground values keep the separately frozen [transport oracle](../ground-transport/README.md). A wide-ID arm proves distinct identities beyond float32's exact integer range.

Source discovery separates immutable building facts from live physical state: `building_of` keeps authored identity, live `structure_owner` requires a present body, and remembered knowledge keeps the state it actually observed. Ordinary remains carry an immutable authored source; genuinely dynamic bodies have none. Appearance lookup and static suppression read that source directly.

The windowed movement producer keeps its original physical subset: it filters whole building owners and ordinary props, then gives the retained IDs the dense namespace the original filtered array used. It introduces no runtime conversion or partial owner.

## Cutover of existing scenarios

Eight older scenario tests built garrison-capable houses as ordinary props, and world construction correctly refused them. Their builders now use the shared test map-preparation owner, with bodies, IDs, rules and assertions unchanged. There is no runtime legacy-house fallback. The frozen foliage corpus had the same gap; `foliage-cutover-inputs.json` authors its two original maps through the same owner, and production-Wasm parity passes both arms exactly.

The scripted village attack still enumerated `map.props` for houses, so it never issued its bombardment. It now selects each garrison aggregate's physical owner part. The frozen village regression fails with the old caller and passes with the fix: the first blue ground attack is at tick 1051, tanks 4/5, point [975,752,0]. Comparing the original and current production village through 3602 ticks, publications first diverged at 1051 before the fix; after it they match through 1300 and all 14 ground checkpoints match, ending at digest `7a6d3a148acdc7d5`. This is a bounded shipping-path proof, not universal parity.

## Open

- The physical catalogue is a prototype for existing source boxes. Its classification is not a real regional/category acceptance; source floors, entrances and bays are absent, and completeness consumers must reject those unknowns. No Blender build, appearance binding, model or design was accepted.
- Catalogue-versus-materialized provenance, complete template/source/art G0 and integrated scene verification remain open.
- Native and Wasm differ by one f64 ULP in an initial infantry member's Y coordinate, a seam that predates aggregate ownership. These proofs do not claim universal battle-runtime parity; canonical C00 materialization uses the shared portable evaluator instead.

[The frame proof](visual/README.md) covers what the aggregate looks like through the real renderer.

Raw evidence (logs, snapshots, identities): tag `city-maps-evidence-2026-09-30`.
