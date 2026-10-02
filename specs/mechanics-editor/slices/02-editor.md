# Spreadsheet and explained editing

Implement `/mechanics` as a searchable spreadsheet of exact concrete unit types.
Common gameplay cells are editable; expanding a unit opens grouped complete
controls and linked global weapons. Use the shared protocol from slice 1, with
no filesystem knowledge in the browser. Keep unsaved edits across filtering and
selection, show field errors without losing draft text, and warn before leaving
with a draft. Preview displays changed values and files, affected unit names,
and exact formatted JSON; Save is explicit and disabled until validation passes.

Descriptors own labels, explanations, units, controls and reversible conversions.
Every supported existing gameplay field must have a real explanation, including
nested enums, references, arrays and optional values. Exclude identity/art fields
from editing. Provide overrides and Restore inherited value where applicable.
Landing spread (metres at maximum range) and range are coupled controls;
range edits preserve visible spread and calculate stored angular scatter.
Both raw and computed values remain editable side by side and update each other.
Also convert percentages, shot cadence and full dimensions. Test round-trips,
range/spread coupling, invalid text, inherited restore and shared impact display.

Use actual browser screenshots for the default sheet, searched/expanded rows,
errors, preview and narrow viewport. Compare against the agreed requirements;
run an unprimed screenshot-critique as the last visual check and open the captures
with preview-shots. Human feedback is non-blocking. Review, audit and commit.
