# Frozen ground oracle

Two oracles pin ground behavior from before SA3 changed its storage. Tests regenerate candidate outputs against them; the expected values are never rewritten to fit a new representation.

[`ordered-patches.json`](../../../../fixtures/parity/ground/ordered-patches.json) holds every battle digest and the complete ordered publication of the authored ground scenario over 150 ticks, including withheld consumption, a blue resync and a red replacement snapshot. Revision numbers, channel bytes, cleared bytes and global cell order are all part of the contract.

[`foliage.json`](../../../../fixtures/parity/ground/foliage.json) holds public and side-cleared foliage from village and sensors, as column, row, canopy height and attenuation for each non-open cell. Open cells are exactly zero. The sparse export must match these records exactly.

These oracles judge storage and sampling parity, not map artwork. The [representation seams](seam-evidence.md), the [bounded sampled-word proof](wordstream/README.md) and the [GPU capability checkpoint](gpu-capability-evidence.md) build on them.
