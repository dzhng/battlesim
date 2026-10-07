# 18 Disabled cards

**Unlocks:** the 86 deferred cards (aircraft, rotorcraft, artillery, air
defence, drones, deferred ground and infantry) as recognisable models, still
unbound and undeployable. Runs in parallel with slices 15–17 once the pilot
(14) sets the bar, in three lanes: aircraft, rotorcraft, support and ground.
Each lane collects its families' references first (README
[References](../README.md#references)).

Today they are family blockouts from `roster/disabled_placeholders.py`, and
the 86 cards point at only 52 distinct files (F-22 = F-35; Ka-52 = Mi-24 =
Mi-28 = Z-10; …). The authored T-14, T-15, Type 15, BRM-3K, Jaguar and
Challenger 3 models are referenced by nothing (`source_path` names the
placeholders), and `challenger/…challenger_3.glb` is byte-identical to the
Challenger 2 file. Every card gets its own model; start from the authored ones
where they exist. The disabled gate (`crates/sim/tests/catalog.rs:15`) checks
only ids and strings: make it check that each `source_path` exists and
validates, and that `model_status` is one of the registry's values.
Every vehicle card also gets its own wreck (slice 11), ready for when its
mechanics land.

1. Rebuild each family from its slice 09 references with `vehicle_parts.py`
   and slice 13 materials. Aircraft and rotorcraft need their own parts
   (canopies as `glass()`, intakes, pylons, rotor hubs and blades, landing
   gear), added to the shared library or an `aircraft_parts.py` beside it.
2. The class budgets from slice 14 apply until their mechanics
   define how they are drawn; aircraft use the MBT budget as a ceiling.
3. They stay out of `assets/catalog.json`; the disabled GLB gate keeps passing;
   `fixtures/units/model-manifest.json` status moves from `source_authored`
   to whatever the registry uses for reference-built (add the value if none).
4. **Icons** (user, 2026-10-06). Each disabled card gets a silhouette icon
   derived from its rebuilt model by the same owner as every other unit icon
   (`packages/scene-assets/src/icons.ts`, `silhouette.ts`, written by
   `asset icons`), reading the disabled models through
   `fixtures/units/model-manifest.json`, so they are generated, never drawn,
   and `asset check` fails a missing or stale one. The purchase picker
   (`web/src/battle/present/purchasePicker.tsx`, which today shows an
   unavailable card as text only) shows the silhouette on unavailable cards,
   styled as unavailable. The picker finds icons through a unit type today
   (`UNITS.has(c.id)`), which a disabled card lacks; key the icon by card id
   instead, for every card. Load [`game-ui`](../../../.agents/skills/game-ui/SKILL.md)
   before changing the picker. Test first: a disabled card without its icon
   fails the icon check.
5. Verify each with a Blender or workbench drop (`/workbench`, drag the GLB),
   compare-screenshots against references, screenshot-critique unprimed, last,
   and a grouped non-blocking user preview.

When this lands, close the spec with close-spec.

## Delegated

Part sizes within the references and budgets; the order of families within
each lane; the shape of the shared aircraft parts module.
