# C02: per map fog cell

**Depends on:** C00. C01 is the planned neighboring aggregate pass; fog ownership has no aggregate dependency. **Kind:** slice.

## Question
Is fog resolution owned by the map everywhere?

## Contract it unlocks
`MapDefinition.fog_cell_m` becomes required, and `sensors.fog_cell_m` is **deleted**. The foliage grid follows the map's cell. The village and labs carry 8 m.

## API seam
`contract::map`, `sim::visibility`, `sim::world::forest`, exports.

## What the human can run or see
A street-visibility probe on S1's generated urban plot at G0's cell.

## Verification
- Village digests unchanged.
- Native and wasm replay.
- Foliage agreement.
- Record the intentional fixture/config identity change separately from outcomes.

## Delegated to the implementer
Internal plumbing only. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Village at exactly 8 m for this schema-only cutover. Full-extent delivery at every resolution must pass G0/C07 before playable release; 8 m does not exempt a map. C56 names later geometry/identity changes.

## Feedback that would change this slice
Unacceptable street visibility or snapshot delivery at the chosen cell size reopens the measured fog choice at G0.

## Systems implementation

`MapDefinition` requires the resolution; sensor rules retain the visibility target height. `WorldGeometry` refuses nonfinite/nonpositive cells before constructing foliage, and `Battle` uses the same map value for its visibility grid. Existing catalogue/analytic inputs explicitly retain 8 m. Changing resolution changes map/scenario identity; deleting the former sensor field changes config identity. This is a named schema cutover, with unchanged existing battle outcomes.

The [cutover receipt](../assets/map-fog-ownership/README.md) pins sources and runner results. The public Battle regression exercises different resolutions on partial edge cells and checks both observation and populated foliage exports. It failed on the original sensor-owned grid and passes on the map-owned grid. The invalid-resolution regression was falsified by removing finite admission, which reached an arithmetic failure instead of the named input rejection.

Original terrain, foliage, learned-ground and observation oracles remain immutable. Tests adapt only their input field location (or supply the original 8 m where an archived map has no rules); complete expected output records and digests remain the frozen references. Full-size street/delivery acceptance remains at S1/G0/C07. The integrated check/verify gate runs before merging the representation wave.
