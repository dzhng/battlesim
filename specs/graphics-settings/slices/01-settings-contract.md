# Slice 01 — Settings contract and quality profiles

## Contract unlocked

A browser-owned graphics settings module can load/save exactly four modes, resolve a concrete effective tier, and expose immutable quality profiles without touching simulation state.

## API seam

Create one owner module near the battle-lab settings owners. Define `GraphicsMode = "auto" | "low" | "medium" | "high"`, an effective tier type, persisted auto-detect metadata, strict validation, `get/set/subscribe`, and a pure `resolveAutoDetect(results)` function. Use a new localStorage key and treat absent/invalid records as first-launch Auto-detect. Profiles must preserve bloom, cast lights, authored sun/sky, and effect mood; only approved render-cost controls vary. Encode the measured candidate values and conservative grass multipliers as data, not scattered constants.

The resolver accepts complete tier results with finite average FPS and 1% low FPS. It returns the highest tier with average FPS at least 45 or an unusable result; the 1% low is retained as evidence but does not gate selection.

## Human-visible artifact

A small settings probe or route may show the loaded mode, effective tier, and serialized record. It need not expose final UI yet.

## Verification

Write tests first for missing/malformed/unknown storage, round-trip persistence, threshold edges, unusable results, profile monotonicity, mood invariants, and no simulation/digest inputs. Run the narrow web test file and typecheck for touched modules.

No screenshot gate is required unless the probe is visual; if one is added, run `screenshot-critique` last.

## Delegated freedom

Module filename and internal store mechanics may follow the sound store. Profile representation and naming are delegated only where behavior remains identical to this contract.
