# Slice 01 — Freeze the baseline and add the graphics test

## Contract unlocked

Browser and future native clients consume one versioned workload and report shape. The workload names the exact full menu reel, menu catalog/maps, encounters, seeds, warm-up, camera shots, resolution, quality and timing boundaries. The browser Settings page runs it and shows average FPS, 1% low, absolute minimum, maximum, raw intervals and a chart. The current synthetic `/benchmark?preset=city-contact` remains unchanged. Before this slice closes, canonical reference evidence must use fixed workload time, the menu's 0.5 s subject smoothing and an explicit camera-plate composition decision; a live run cannot stand in for that frozen cross-engine stream.

## Seam and ownership

Extend the existing benchmark/report owners under `web/src/battle/benchmark/` and the Settings page in `apps/battle-lab/src/MainMenu.tsx`. Read scene identity from `apps/battle-lab/src/menuReel.ts` and `fixtures/menu-backdrop.json`; do not duplicate menu content. The user-facing route records its explicit presentation contract, while cross-engine evidence must be exported as `battle-presentation-capture/v1`; neither may use `prefers-reduced-motion` or silently inherit the live menu's simulation-clock pacing.

Derive FPS from displayed-frame intervals: mean from total elapsed time, 1% low as 1000 divided by the mean of the slowest ceil(N × 0.01) valid frame intervals (this is distinct from inverse p99), and absolute minimum/maximum from observed displayed frames. Preserve raw samples for the chart and label offscreen, submission and GPU timings separately.

## Runnable artifact

On Mac, a production browser build exposes Settings → Graphics Test at `/graphics-test`. One action runs the complete menu reel and ends on a chart and summary. An ignored-evidence manifest and JSON report record build, asset, browser, GPU, resolution, quality, workload identity and samples. The existing synthetic benchmark still runs through its existing route and report.

## Verification

Write report and manifest tests first. Test percentile/min/max edge cases, empty/partial runs, deterministic manifest identity and canonical camera replay. Run focused web tests, typecheck and the menu/benchmark scenes; serialize GPU work with the repository lock.

Capture named reel cuts before and after. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the same crops; judge only camera, scene identity and visible loading/veil state here. Lighting, materials and Godot rendering are out of scope. Finish with [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) and show captures with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This review is non-blocking; proceed after a short response window and record any correction.

## Delegated choices

Internal report types, chart library/CSS and evidence filename are delegated. Workload identity, timing definitions, menu ownership and preservation of the synthetic benchmark are fixed.

## Must stay green

Existing benchmark tests/scenes, menu scene identity validation, browser build, simulation digest/replay tests and current Settings behavior.
