# 10: Three roster cards, shop order, voices, tutorial

**Contract.** (R1, G, D7–D10, P13–P16) Each faction can buy its supply heli:
- **Cards:** us "CH-47F Supply", europe "Chinook HC6 Supply", eastern
  "Mi-8AMTSh Supply".
- **What each card is:** cost 150, supply stock 500, deploy 3 s / pack 1 s /
  drop 2 s, roster category `sup`, role `logistics`. It keeps the transport's
  door guns, has no seats, uses the transport's appearance, and authors a rotor
  radius.
- **Shop:** in LOG, after the truck.
- **Voices:** the faction's transport-pilot set.
- **Tutorial:** the LOG line reads "supply trucks and helicopters".

**Seam.**
- `fixtures/units/roster/{us,europe,eastern}.json` (P14):
  - each transport row (`us_ch_47_chinook_ch_47f`, `europe_ch_47_chinook_hc6`,
    `eastern_mi_8_hip_mi_8amtsh`) becomes an abstract `<id>_airframe` holding
    everything but `carriage`;
  - the transport row extends it and adds `carriage`;
  - the supply row extends it and adds name, description, cost, roster, roles
    and capabilities, plus `rotor_radius_m` (measured from the model's rotor
    nodes: the swept circle about the hull centre, P6).
  - The resolved transport rows in `fixtures/catalog.json` must be
    byte-identical.
- `fixtures/catalog.json` re-bless (`BLESS_CATALOG=1 cargo test -p sim --test sim catalog::`).
- `fixtures/voices.json`: assignments to `us_` / `de_` / `ru_transport_helicopter_pilot`.
- `web/src/battle/present/purchaseFilter.ts::shopOrder`: ground movers sort
  before aircraft within a tab (P13).
- `apps/battle-lab/src/Tutorial.tsx` (~25 and any line that says only "trucks").
- `packages/scene-assets/blender/catalog_frames.py::_variant`: accepts several
  unit types drawing one appearance when they agree on hull and mounts, and
  refuses otherwise (P15).
- Card icons, `assets/icons/units/<card>.svg`, rendered by the asset CLI. This
  needs `git lfs pull` of the three transports' runtime bundles first.
- Run `asset check`: `vehicleFit` passes because the mounts are kept.

**Run / see.** In a skirmish, the LOG tab shows the truck then the supply heli.
Buy it, watch it land and serve. The Armory card reads like the truck's.

**Verify (test first).**
- The catalog's generated guards cover each new row (sets up, fires each mount,
  moves, admitted). Tests name no roster row.
- `web/tests/purchaseFilter.test.ts`: with two synthetic cards, a card that
  drives sorts before one that flies in the same tab.
- `web/tests/voiceCatalog.test.ts` passes (every playable unit has a voice).
- A Python check, or the exporter's own refusal, for `_variant`: two types with
  equal hull and mounts share an appearance; unequal ones are refused.
- A pinned-seed skirmish probe per faction: the AI opens with a truck (slice 01
  holds).
- `unit-roster` and `armory` scenes and the shop baselines: run
  `compare-screenshots` before against after, then re-bless the intended change
  yourself.
- **Last check:** an unprimed `screenshot-critique` on the LOG tab and the card.
  Show the user inline; non-blocking.

**Digest.** Named:
- catalog identity grows (type indices renumber);
- the game catalog's low hover rises (D5: the CH-47 is 5.43 m tall, so ~6.1 m
  becomes ~7.4 m) for every heli that sinks at a supplier;
- the skirmish corpus moves where slice 09's emergency fires.

Re-record parity, capture and corpus fixtures once. First prove that only these
three causes move them.

**Delegated.** Exact ids for the supply rows (pattern
`<faction>_<family>_supply`) and the airframe rows.

**Stays green.** Every catalog guard; the tutorial scene; scene-asset checks;
the voices lock.

**Feedback that would change it.** Card names or icon look.
