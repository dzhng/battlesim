# C14: template damage states

**Depends on:** C32; terminal geometry contract retained from Q4. **Kind:** slice.

## Question
Do reusable intact/ruin/gutted template states fit the same building geometry and accepted damage rules?

## Contract it unlocks
The C13 source exporter and C32 library builder evaluate patched damage inputs for each reusable recipe, producing ruin states for ≤6 floors and standing burnt/gutted states above 6. The physical ruin height uses the existing provisional Q4 ratio/clamp pending C50. A C50 ratio change must rerun this terminal bake/fit gate, publish a new appearance hash and rerun fit before C54/C51; tuning hp alone needs no rebake. Intact physical catalogue/map identity stays unchanged; resolved rule/config identity names the changed terminal bounds. Damage remains a simulation event; appearance resolution chooses the state the side has observed.

## API seam
Offline family/graph patch scripts → C13 state source placements → C32 packing/fit validator. C42/C43 own damage transitions; the template resolver never reads hidden live state or invents a terminal body.

## What the human can run or see
Intact/ruin/gutted contact sheets and sim-bounds overlays by physical floor class, including the new homes, towers and industrial templates.

## Verification
- Byte identity and terminal-state coverage for each released template.
- Ruin bounds fit accepted terminal geometry; gutted bounds retain standing height.
- The 9+ highrise layout category does not change the >6-floor gutting threshold.
- Compare terminal-state silhouettes/fit against Q4 geometry and intact template evidence with compare-screenshots; run unprimed screenshot-critique last; preview-shots non-blocking.

## Delegated to the implementer
Breach pattern/rubble distribution within physical bounds. Damage thresholds/timing and new states are not delegated.

## Must stay green
One sim damage owner and side-knowledge timing.

## Feedback that would change this slice
Rejected terminal fit changes its recipe; C50 alone tunes durability/ruin coefficients.
