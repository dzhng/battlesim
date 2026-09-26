# Frame cost

One row per slice: 1920×1080 on this Mac (M5 Pro, hardware Metal, Chromium). The method comes from slice 00 before slice 10 lands, and from the benchmark short run after. No budget until slice 27 (30 FPS floor at the default camera).

| slice | method | p50 ms | p95 | p99 | GPU passes | buffer MiB | texture MiB | publication B/tick | note |
|---|---|---|---|---|---|---|---|---|---|
| 00 | `village-perf` probe, 10 s per camera, redraw forced every frame | 8.3 | 9.2 | 9.3 | — | 37.3 (20 buffers) | not sized (2 textures) | 12,168 per publication | Before state, flat-shaded village. All three cameras (strategic, default, ground) sit at the display's 120 Hz cadence, so they are indistinguishable. Simulation runs at 30.6 Hz. `timestamp-query` is available in headless Chromium on Metal; per-pass timing needs `BattleFrame` (slice 12). |
