# Unactivated centerline prototype

This was C65 research, never compiled into the contract or used by roads, forests or rivers. The live system kept straight authored runs.

A centripetal Catmull–Rom curve wandered 29.8, 70.2 and 32.7 m from the village control runs, too far for a 12 m road. The alternative rounds each control locally with paired cubic Bézier segments, keeps the straight middle and every authored point, and samples at most every 2 m under a caller allowance. Four narrow native tests passed; removing the corridor cap and ignoring the sample allowance each produced a recorded failure.

Native/Wasm parity, production consumers, total admission, village outcomes and visual gates were not attempted. The candidate source (`curve.rs`, `curves.rs`) and the rejected global-handle variant are only in tag `city-maps-evidence-2026-09-30`. Resume through [C65](../../slices/C65-round-centerlines.md).
