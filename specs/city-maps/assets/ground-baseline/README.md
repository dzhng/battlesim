# Frozen ground oracle

These outputs pin the existing ground behavior before SA3 changes its storage. The source commit, source hashes and original wasm hash are in `identity.json`. Regenerate candidate outputs against these files; never rewrite the expected values to accommodate a representation change.

`ordered-patches.json` captures every battle digest and complete ordered publication for the authored ground scenario over 150 ticks, including withheld consumption, a blue resync and a red replacement snapshot. Revision numbers, channel bytes, cleared bytes and global cell order all remain part of the contract.

`foliage.json` captures public and side-cleared foliage from village and sensors. Its canonical non-open records use column, row, canopy height and attenuation. Open cells are exactly zero. Canonicalization removes blank records from the old dense export without changing any values; the candidate's sparse export must match the frozen records exactly.

The production ground scene's original captures remain in the gitignored `throwaway/city-spike/sa3/production-before/`. They judge storage and sampling parity; they do not approve deferred map artwork.
