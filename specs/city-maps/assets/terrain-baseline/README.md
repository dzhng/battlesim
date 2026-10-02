# Exact terrain representation

The terrain oracle (`fixtures/parity/terrain/queries.json`, at tag `data-files-before-prune-2026-10-01`) held the original sampled heights, normals, first ray hits and classifications over the relief, water and bridge fixture. It left the tree once the bounded representation was the only one; the analytic tests in `crates/sim/tests/world_geometry.rs` state the expected answers. The village trace kept every tick's battle digest.

The bounded terrain candidate passed those queries. Native allocation arms at 12, 15 and 18 km, including an end-to-end narrow river, ran serially; their peaks include the then-dense foliage owner, so they are not terrain-only storage. Apple Metal height readback through the production page reader and triangle rule matched, and every GPU resource was released. Paired original and candidate frames at matched framing showed no change; those ground frames predate SA3's scar-cache correction.

This is a physical and resource proof, not city artwork or world-scale acceptance. The [SA1 verdict](../../spikes/SA1.md) owns the conclusions.

Raw evidence: tag `city-maps-evidence-2026-09-30`.
