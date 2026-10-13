# 02 — HighAir motion and vision boundary

**Contract:** persistent fixed-wing hulls fly at shared HighAir height, have no
ground footprint, do not use ground traffic/cover/treads, circle at a location
order, and stay inside deterministic map bounds. Aircraft never contribute to
Ground fog or Ground identification; they can observe HighAir and LowAir by
their authored sensor rows. Helicopter motion remains unchanged.

**Seam:** `sim::units`, `movement::air`, `navigation::air`, `sensing`,
`visibility`, and presentation layer filters. HighAir routing is one owner;
do not fork a browser flight model.

**Runnable proof:** `/lab/high-air` flies a fixed-wing unit over buildings,
trees, and ground fog beside a helicopter, showing separate layers and a stable
circle order.

**Checks:** no Ground fog changes, no ground collision/occupancy, HighAir and
LowAir target eligibility, deterministic map-edge/boundary movement, replay
digest equality, and WASM frame round-trip.

**Visual gate:** altitude/silhouette crop only; run `game-ui`,
`compare-screenshots` when a reference exists, `screenshot-critique` last, and
offer `preview-shots` non-blockingly.

**Scope firewall:** no weapons, radar, ingress delay, dogfight, fuel, or audio.
