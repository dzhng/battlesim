# 13 · The roster's carriers get their seats

**Owns:** D3, D4, L9 (roster). **Depends on:** 06–09, with their behaviour tests in place (`specs/done/unit-roster/capability-admission.md:38`), and 05.

## Contract
`capabilities.carriage.seats` on the roster rows, exactly as D3:

| Seats | Units |
|---|---|
| **16** | CH-47 (US, Europe), Mi-8, AW101, NH90 |
| **8** | M2 Bradley, Stryker ICV, Stryker Dragoon, LAV-25, ACV-P, BMP-2M, BMP-3, BTR-82A, T-15, ZBL-08, Boxer APC, Boxer RCT30, CV90 Mk IV, CV9040C, Puma, VBCI, UH-60, UH-1Y, Z-20, Mi-35 Hind |
| **3** | HMMWV, Tigr-M, VBL, M3 Bradley, Stryker RV, BRM-3K, MH-6 Little Bird |

- Everything else carries no one, including the three supply trucks (D4) and the air-defence vehicles.
- Where a family shares a profile through `extends`, put seats on the most specific row that is a carrier, so a non-carrier sibling doesn't inherit them.
- Remove "passenger carriage not implemented" from every description: 18 rows, including two air-defence vehicles that stay without seats. Rewrite each description without it.

## What a human can see
- The purchase picker shows seats on carrier cards, if the card has a capability line; check `purchasePicker.tsx`.
- In a skirmish, any carrier can be boarded.

## Verification
- **No test pins the seat table.** AGENTS.md: tests test mechanics, not configuration; a roster is data meant to change.
- Instead, the orchestrator checks the data once by hand against D3, using a scratch listing of every unit's resolved `carriage.seats` from `fixtures/catalog.json`, and records the result in the slice's choices entry.
- The mechanics are already pinned by slices 06–09's tests, which run on test carriers.
- Re-bless `fixtures/catalog.json` and the parity records.
- No outcome change: nothing boards until a player or the AI orders it.

## Delegated
Wording of the rewritten descriptions.

## Must stay green
Catalog admission, the menu reel, the purchase picker scenes.
