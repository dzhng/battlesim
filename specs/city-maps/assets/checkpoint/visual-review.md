# Ground checkpoint review

## Target and comparison

Retain the original Village physical opening and learned side-specific ground through the structural changes. The historical screenshots are a parity reference, not an approved artwork design. The fixed scene framings, viewport and seed are unchanged. Actual shipping publications and ground cells are separately checked by the [script owner regression](../building-aggregate/integration/script-owner/README.md).

All 17 ground captures were frozen and compared with the historical production-before captures. `ground-comparison.json` retains full grayscale, edge and content telemetry. Largest pixelmatch ratio is 0.00051 (red side inspector); village default, field and full blue inspector have zero pixelmatch mismatch. These numbers locate changes, not prove art quality. Root reviewed all three full-state sheets plus the native village ground frame.

## Fresh independent critique

An unprimed agent inspected all 17 native full captures and all 17 centre crops. Its findings are retained below. Initial track crops missed the lower track bands; six additional feature crops corrected that evidence gap. The critic then inspected those crops and five corresponding historical full captures. It found no candidate-specific visible regression in those five comparisons. This is a bounded review and does not classify every older finding’s provenance.

| Visible finding | Evidence and scope |
|---|---|
| The full blue inspector is almost entirely a blue-grey striped surface; units/terrain are unjudgeable. | High confidence, `frame-village-blue`; also present in the historical capture. Do not use it as composed-scene evidence. |
| Bright vegetation remains across crater interiors and rims. | High confidence, field/barrage full frames and crops; also present in historical captures. |
| The dense crater field has abrupt rectangular boundaries and repeated aligned bowls. | High confidence, field full frame; historical fixture has the same stamped layout. |
| Large blank red/white bars sit away from infantry; white bars appear among trees/fields. | High confidence, ground/village-ground full frames and corrected crops; also present historically. |
| A repeating blue-grey strip behind the trees reads as a patterned wall. | Medium/high confidence, shallow full ground frames; also present historically. |
| Shallow road craters read as flattened dark smears. | Medium confidence, village-ground; also present historically. |
| Tracks are faint, with vegetation visible through them. | Medium confidence, native full tracks and corrected crops. Both bands are continuous over the supplied span; enabled/disabled states differ as expected. |
| A narrow crater contour resembles a layered cutout. | Medium confidence, red inspector crop; weak at native full scale and not independently classified against history. |

The angular dark regions in blue/red inspectors attach to the building and read as building shadows; the critic found no convincing shadow/fog confusion there. Corrected infantry crops show complete bodies/rings with no new clipping or missing infantry.

## Limits and disposition

This accepts only the restored physical/learned-ground checkpoint and bounded historical parity. It does **not** accept the artwork or these existing presentation defects. They remain work for the visual specialist and evidence rig; no presentation fix was attempted during the stopping checkpoint.

Track feature crops cover x120–1860/y700–1050, excluding outer endpoints/vehicle attachment. Large crops were reduced by the viewer to 2048 pixels wide; they support shape and continuity, not exact enlarged-pixel inspection. The blue inspector framing remains invalid for scene composition.

The village craters, historical/current pair and separate blue/red inspectors were opened together in Preview for the user. Local full/crop/diff evidence is preserved under `/Users/david/dev/battlegame/throwaway/city-spike/checkpoint-ground-review`; capture hashes and crop bounds are committed.
