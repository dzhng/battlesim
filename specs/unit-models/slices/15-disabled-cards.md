# 15 Disabled cards

**Unlocks:** the 86 deferred cards (aircraft, rotorcraft, artillery, air
defence, drones, deferred ground and infantry) as recognisable models, still
unbound and undeployable.

Today they are family blockouts from `roster/disabled_placeholders.py`; BRM-3K,
T-14/T-15, Type 15, Challenger 3 and Jaguar have authored family models.

1. Rebuild each family from its slice 09 references with `vehicle_parts.py`
   and slice 08 materials. Aircraft and rotorcraft need their own parts
   (canopies as `glass()`, intakes, pylons, rotor hubs and blades, landing
   gear), added to the shared library or an `aircraft_parts.py` beside it.
2. No budget beyond the class budgets from slice 10 until their mechanics
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
