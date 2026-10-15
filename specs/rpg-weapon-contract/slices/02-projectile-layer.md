# 02 — Immutable projectile layer and interception

## Contract unlocked

Every projectile carries the firing unit's `AltitudeLayer` from launch for its entire lifetime. Interception uses that stored layer to select `targets` range/scatter. Projectile height remains physical flight state only.

## Seam and ownership

- Add layer to `flight::Launch`/`Projectile` and the battle round/replay state as needed.
- `Battle::fire` copies `shooter.layer()` at launch.
- `arm_missile_interceptors` selects loaded mounts containing `can_intercept` weapons; no unit `air_defense` gate and no z-derived layer.
- Radar contact logic reads `radar`, never AA role metadata.

## Tests and gates

- Ground ATGM stays ground-layer while physically above air thresholds.
- Low/high-air launches retain their source layer while descending/ascending.
- Interceptor range selection follows stored layer.
- Any interceptable projectile can be acquired by a compatible `can_intercept` mount.
- Replay/digest parity covers the stored layer.
- Existing guided seeker/proximity-fuse tests remain green.
