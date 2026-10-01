# Map-owned fog cutover

The [C02 slice](../../slices/C02-per-map-fog-cell.md) owns the contract and the remaining full-size acceptance. Moving the fog resolution into the map left the original terrain, foliage, ground and observation oracles unchanged; only their test inputs carry the resolution in the map. A native battle regression observes both the visibility field and populated foliage. Native tests, the frozen publication, the village replay and rebuilt-wasm consumers agree; this proves ownership and 8 m parity, not a full-size active battle or street resolution.
