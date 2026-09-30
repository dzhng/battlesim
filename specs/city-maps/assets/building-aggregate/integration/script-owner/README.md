# Script aggregate owner regression

The first full scene gate found missing exclusive craters and missing watch contacts. The scripted village attack still enumerated `map.props` for garrison houses, after C01 moved houses into `map.buildings`. It therefore never issued its historical bombardment.

The fix selects each garrison aggregate’s referenced physical owner part. The frozen original village regression fails with the old caller and passes with the fix: the first accepted blue ground attack is at tick 1051, tanks 4/5, point [975,752,0]. The fixture is pinned separately from live tuning.

Actual original/current production village factories, Wasm decoding, commander and battle were compared through 3602 ticks. Before the fix, own publications first diverged at 1051. After it, own publications match through 1300 and all 14 ground comparison checkpoints match, including final digest 7a6d3a148acdc7d5. This is a bounded shipping-path proof, not a claim of universal floating point or all-scenario parity.

The initial full gate also recorded one cleanup cycle with 2688 extra allocated bytes. Two focused runs subsequently passed the unchanged assertions; the second records actual GPU buffer allocation census. No renderer fix or tolerance change is claimed. The final full gate must pass before push.

The original decoder/input and corrected script are retained as snapshots; `identity.json` pins both actually executed original/corrected Wasm binaries. Reproduction stages the probes at their original `throwaway/city-spike` paths, builds the pinned original checkout and `27e03c4` with the corrected script, and uses the matching decoder for each layout. No claim is made to have retained the discarded bad binary; its failing rows and rejected source are recorded instead.
