# Team skirmish slice 00 probe

| pairing | admission rate | refusal reasons | prepare time per size | estimated opening field time (1 s dispatch per entry, ~9 opening units/player) | arms spacing (≥150 m) | river-mouth avoidance |
|---|---:|---|---|---:|---|---|
| 1v1 | 2/7 (29%) | 1× fairness >15% (`test_aa` → `field-0-1`, 306.4/541.1 s); 1× fairness >15% (`test_aa` → `field-6-0`, 495.6/302.7 s); 1× fairness >15% (`test_aa` → `field-9-0`, 367.1/167.7 s); 1× fairness >15% (`test_supply` → `east`, 127.8/101.0 s); 1× fairness >15% (`test_tank` → `center`, 44.3/32.6 s) | S floor: 627.4 ms mean | 9 s (estimate) | yes | yes |
| 2v3 | 0/7 (0%) | 1× Red entry body; 2× fairness >15% (`test_aa` → `east`); 2× fairness >15% (`test_aa` → `field-7-1`); 1× fairness >15% (`test_supply` → `west`); 1× fairness >15% (`test_tank` → `center`) | L floor: 709.9 ms mean | 9 s (estimate) | no | yes |
| 3v3 | 2/7 (29%) | 1× Blue entry water/slope; 1× Red entry body; 2× fairness >15% (`test_aa` → `field-7-1`); 1× fairness >15% (`test_supply` → `west`) | L floor: 889.7 ms mean | 9 s (estimate) | no | no |
| 5v5 | 2/7 (29%) | 1× Blue entry water/slope; 3× fairness >15% (`test_aa` → `field-0-1`/`field-3-1`); 1× fairness >15% (`test_supply` → `west`) | XL floor: 4,205.8 ms mean | 15 s (estimate) | no | no |

The verdict is **Fail**. The 1v1 probe admits 29% of the seven corpus rows, while 2v3 admits none and the larger pairings remain at 29%; the multi-entry results are not in the same range as the current 1v1 assumption. The dedicated arms also fail the spacing or river-mouth checks on the larger pairings. Slice 04 should be resliced before contract work, as the slice rule requires.

## How measured

- Probe source: `crates/mapgen/examples/slot_probe.rs` (throwaway only). It keeps the production `[blue, red]` entry contract unchanged, derives 1/2/3 entries per side locally, and appends temporary `CountryRoad` arm strokes to a cloned map. The arm placement uses a deterministic, probe-local stream named `entries`; the centre entry remains the existing spine entry.
- Admission uses `sim::encounter::legality::stands` and `sim::navigation::plan`/`route_time`. For each representative foot, tracked and wheeled unit and objective, each side contributes its best (minimum) travel time over its entries. The unchanged 15% comparison is applied.
- Generation and `PreparedMap` construction are timed with `Instant`; values are mean milliseconds per corpus row at the D18 floor (generated rows ratchet up to their authored size). Opening times are estimates: `max(players × 9 / entries)` seconds, with entries 1/2/3 for 1/2/3+ players.
- Corpus: `fixtures/skirmish-corpus.json`, revision 1, all 7 rows and seeds 1–7 (three generated rows and four saved fixture rows). Saved rows use the probe's synthetic three-objective sites; no production map or contract files were changed.

Rerun:

```bash
CARGO_NET_OFFLINE=true cargo run -p mapgen --release --example slot_probe
```
