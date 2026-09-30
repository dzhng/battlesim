# Unactivated centerline prototype

This checkpoint preserves C65 research and a tested candidate. It is not compiled into the contract or used by roads, forests, or rivers. The live system retains straight authored runs.

The centripetal Catmull–Rom trial wandered 29.8/70.2/32.7 metres from the village control runs, too far for the 12 metre road. The alternative rounds each control locally with paired cubic Bézier segments, preserves the straight middle and every authored point, and samples at no more than two metres under a caller allowance. Four narrow native tests passed; removing the corridor cap and ignoring the sample allowance each earned a recorded failure.

This is preliminary research: native/Wasm parity, production consumers, total admission, named village outcomes, and visual gates remain pending. `curve.rs.snapshot` and `curves.rs.snapshot` are the final candidate and tests. Other source snapshots preserve the rejected global handles and initial straight scaffold. The logs and literal falsification results retain their original verdicts.

Resume through [C65](../../slices/C65-round-centerlines.md); do not copy the prototype into production without that slice’s remaining gates.
