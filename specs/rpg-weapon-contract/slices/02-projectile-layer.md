# 02 — Immutable projectile layer and interception

## Contract unlocked

Every projectile carries the firing unit's `AltitudeLayer` from launch for its entire lifetime. Interception uses that stored layer to select `targets` range/scatter. Projectile height remains physical flight state only.

## Seam and ownership

- Add layer to `flight::Launch`/`Projectile` and the battle round/replay state as needed.
- `Battle::fire` copies `shooter.layer()` at launch.
- `arm_missile_interceptors` already selects mounts by their weapon and has no unit gate (D52); this slice only replaces its "longest air reach" range choice with the stored launch layer.
- Radar contact logic already reads `radar` alone (D52).

## Tests and gates

- Ground ATGM stays ground-layer while physically above air thresholds.
- Low/high-air launches retain their source layer while descending/ascending.
- Interceptor range selection follows stored layer.
- An interceptable projectile can be acquired by a mount whose row lists `missile` at its launch layer (D54).
- Replay/digest parity covers the stored layer.
- Existing guided seeker/proximity-fuse tests remain green.
