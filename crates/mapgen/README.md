# Physical map compilation

The compiler resolves physical template placements into the contract's final map.
It imports no simulation or appearance library. The battle and renderer consume that
compiled geometry; they never reinterpret its plan or look up a template catalogue.

Generation configuration pins the lossless map seed, generator/preset versions,
canonical plan and physical catalogue. Execution limits are separate preparation
policy. The map's content hash identifies physical output independently of its
execution allowance or presentation assets.

The shared identity owner lives in `contract::identity`. A seed is canonical decimal
text at JSON boundaries. Typed field order defines canonical bytes; authored sequence
order is meaningful because implicit IDs occupy the remaining dense namespace. Input
whitespace and object-key order do not change identity. Version labels are supplied
by preparation callers, so this is content identity rather than verified source provenance.

The compiler admits physical buildings, ordinary authored bodies, and ground: roads,
tracks, sidewalks and forests in the contract's shared shapes, which pass into the map
unchanged. Explicit body IDs enter the contract-owned dense namespace. A ground shape's
authored points must lie inside the playable rectangle (a stroke may overhang the edge
by its width). Rivers, land regions, layout generation and source/art fit remain
prerequisites for their compiler arms; a requested unsupported feature produces a named
error rather than disappearing from output.

`MapPlan.size` is the playable rectangle. Admission follows the architecture envelope
in the city-map scale policy; it defines no release presets or rendered surroundings.
Physical box bounds use the template contract's numeric evaluator for admission.
Simulation collision arithmetic and its remaining cross-runtime proof are independent.

The library's complete outcome is shared by the CLI and WASM. A refusal returns
diagnostics and no map. Successful file preparation writes the final map and the
shared `MapSources` envelope. Supplied request/catalogue receipts hash exactly the
bytes compiled and assert no Git history; generation identity and stdout outcomes
are unchanged. Acquisition and catalogue publication belong to C09/C60. The required
execution limits cover authored parts, emitted bay positions and ground points (polygon
vertices plus rounded stroke samples) before materialization.
They do not claim a bound on all input bytes, terrain, navigation, runtime trees or
the complete battle's memory.
