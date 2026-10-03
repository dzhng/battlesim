# Parameter policy

This is the complete source-constant partition from the frozen 76-row inventory, verified against named owners during slice 01 before extraction. P = construction/admission policy; W = finite advanced work policy; I = implementation correctness/discretization/physical contract. P/W values move into their current preset rule groups with exact initial values. Never infer safe ranges from a symbol's name; inspect readers, validate source semantics and record corrections here before changing controls. All current preset leaves separately receive editable/managed/excluded/correctness roles in the native field inventory. The historical count is not a live slider count.

Topology-adjacent morphology policies remain subject to final physical/topology checks. Numerical rounding, overshoot, degeneracy, mathematical/proof and contract sampling settings have no disable control. Similar symbols from different owners are not aliases by name; keep distinct concepts distinct and merge only an actually shared concept.

| Owner | Symbol | Initial value | Role | Owning rationale |
| --- | --- | --- | --- | --- |
| `crates/mapgen/src/joints.rs` | `MEET_M` | `0.5` | P | Ends this near each other are one place. |
| `crates/mapgen/src/joints.rs` | `OVERSHOOT_M` | `0.25` | I | An end stops this far past the middle of the road it joins, so the two centrelines cross whatever a centimetre of rounding did to either. |
| `crates/mapgen/src/joints.rs` | `NEAR_M` | `2.5` | P | An end joins a carriageway whose edge is this near it. |
| `crates/mapgen/src/joints.rs` | `REVERSE_COS` | `-0.996` | I | Two ends that turn back on each other this exactly (about 5°) are not welded: the shared centreline has no bend to give them. |
| `crates/mapgen/src/joints.rs` | `THROUGH_COS` | `0.866` | P | At a junction a wider road also ends on, two narrower ends within this of straight (30°) are one road through it; otherwise the wider road carries on down one of them ([`carry`]). |
| `crates/mapgen/src/joints.rs` | `TAIL_SPARE_M` | `1.0` | P | An end's last run is at least this much longer than the widest road's half width: the cut of a square end reaches half a width back, and a corner behind it is to stay round. A turn nearer an end than that is dropped, and no way's width changes that near the corner it was carried round. |
| `crates/mapgen/src/joints.rs` | `CARRY_ROUNDS` | `3` | W | A corner of unlike ways is mended over at most this many rounds: a way changes once a round. |
| `crates/mapgen/src/joints.rs` | `BESIDE_WIDTHS` | `10.0` | P | Two alike streets laid side by side whose ends run past each other by no more than this many widths are one street. |
| `crates/mapgen/src/joints.rs` | `JUNCTION_M` | `25.0` | P | Two ways that each cross a third within this of where they crossed each other are still joined there. |
| `crates/mapgen/src/joints.rs` | `SWING_WIDTHS` | `4.0` | P | An end whose last run is no longer than this many of its widths may be swung to meet a road square, when it has no room to turn before it. |
| `crates/mapgen/src/joints.rs` | `SLANT_COS` | `0.5` | P | A branch within this of square to the road it joins (30°) is left as it comes; one that comes in at more of a slant is bent to meet it square. |
| `crates/mapgen/src/joints.rs` | `SQUARE_WIDTHS` | `1.5` | P | A branch bent to meet a road square runs square for this many of its own widths before the road's edge. |
| `crates/mapgen/src/joints.rs` | `STUB_WIDTHS` | `2.0` | P | A road that stops within this many of its own widths past a road it crossed is cut back to that road: a stub, not a road of its own. |
| `crates/mapgen/src/joints.rs` | `FLUSH_M` | `0.02` | I | A face along another carriageway's edge is covered by it. |
| `crates/mapgen/src/joints.rs` | `REACHED_M` | `0.005` | I | An end is moved no less than this. |
| `crates/mapgen/src/joints.rs` | `ON_MIDDLE_M` | `0.1` | I | An end this near a road's middle stands on it, and has not passed it. |
| `crates/mapgen/src/joints.rs` | `ROUNDING_M` | `0.005` | I | How far a centimetre's rounding may move a point. |
| `crates/mapgen/src/joints.rs` | `ALONGSIDE_COS` | `0.94` | P | A road nearly alongside is not one an end runs into (20°). |
| `crates/mapgen/src/joints.rs` | `GATE_WIDTHS` | `2.0` | P | A road leaves the map square to its edge over twice its width. |
| `crates/mapgen/src/joints.rs` | `SQUARE_SIN` | `0.02` | I | Lines this near parallel (about 1°) have no corner between them. |
| `crates/mapgen/src/joints.rs` | `TRIM_MARGIN_M` | `1.0` | P | A road cut back to a settlement stops this far past the last ground it runs along, and is walked back to it in steps this long. |
| `crates/mapgen/src/joints.rs` | `TRIM_STEP_M` | `2.0` | I | See the owning source for its geometric use. |
| `crates/mapgen/src/joints.rs` | `PIN_ROUNDS` | `4` | W | How many times the ways of a lost joint are pinned and the rest closed again, before the plan's roads are left as they were laid. |
| `crates/mapgen/src/joints.rs` | `SHORTEST_RUN_M` | `1.0` | P | No end is cut back, and no gate laid, to leave a run shorter than this. |
| `crates/mapgen/src/layout/measure.rs` | `OVERRUN_M` | `1.0` | I | A road that ends on another runs no farther than this past its middle. |
| `crates/mapgen/src/layout/roads.rs` | `ON_GROUND_M` | `0.5` | I | A point this near a settlement's limit stands on its ground. |
| `crates/mapgen/src/layout/roads.rs` | `SAME_PLACE_M` | `1.0` | I | Two points nearer than this are one place: no road runs between them. |
| `crates/mapgen/src/layout/roads.rs` | `STRAIGHT_M` | `0.25` | I | A stretch of road whose points lie this near one line is straight. |
| `crates/mapgen/src/layout/roads.rs` | `FORK_SIN` | `0.707` | P | Two roads meet no nearer alongside than this (the sine of 45 degrees). |
| `crates/mapgen/src/layout/roads.rs` | `SIDE_ROAD_TRIES` | `4` | W | How many places are tried for each secondary road a settlement has. |
| `crates/mapgen/src/layout/towns/ground.rs` | `CORNER_M` | `5e-3` | I | Two points nearer than this are one corner: two leaves each work out where a cut crosses the edge between them, a hair apart. |
| `crates/mapgen/src/layout/towns/ground.rs` | `SNAP_M` | `0.01` | I | A cut that passes a corner nearer than this passes through it: the plan is on a centimetre grid, and a corner and a cut a millimetre apart would be two corners to one leaf and one to its neighbour. |
| `crates/mapgen/src/layout/towns/ground.rs` | `SLIVER_M` | `1.0` | P | A line must enter a piece of ground this far to cut it: nearer its edge it runs along that edge. |
| `crates/mapgen/src/layout/towns/ground.rs` | `SHARED_M` | `8.0` | P | A road must run this far through a piece of ground to cut it, and two pieces must share this much edge to be neighbours. |
| `crates/mapgen/src/layout/towns/ground.rs` | `SLIVER_M2` | `1.0` | I | A piece of ground smaller than this is no piece at all. |
| `crates/mapgen/src/layout/towns/ground.rs` | `MEET_M` | `0.5` | P | Two carriageways within this of each other meet. |
| `crates/mapgen/src/layout/towns/ground.rs` | `PLACES` | `6` | W | How many places either side of the one drawn a cut is tried at, to end clear of the junctions on the cuts it ends on. |
| `crates/mapgen/src/layout/towns/ground.rs` | `IN_LINE_SIN` | `0.423` | P | Two cuts that end on a third from its two sides make a crossroads when they run within this of one line (the sine of 25 degrees). |
| `crates/mapgen/src/layout/towns/streets.rs` | `JOIN_OVERSHOOT_M` | `0.5` | I | How far an avenue runs past the middle of the road it meets. |
| `crates/mapgen/src/layout/towns/streets.rs` | `SLANT_SIN` | `0.375` | P | An avenue meets a road no farther off square than this (the sine of 22 degrees): at more of a slant it stops at its last corner before it. |
| `crates/mapgen/src/layout/towns/streets.rs` | `ALONG_SIN` | `0.02` | I | A street within this of a road's line (a sine) runs along it. |
| `crates/mapgen/src/open_country/coverage.rs` | `PROOF_OPERATIONS` | `512_000_000` | I | Proof work only, independent of authored compile limits. Charged geometry predicates and raster reads stop at this envelope; unsupported edited spacing is refused before the shared candidate sampler runs. |
| `crates/mapgen/src/open_country/mod.rs` | `JOIN_OVERSHOOT_M` | `0.5` | I | A lane runs this far past the middle of the road it leaves, so the two centrelines cross whatever a centimetre of rounding did to either. |
| `crates/mapgen/src/open_country/mod.rs` | `LINE_STEP_M` | `20.0` | P | A tree line follows its road by points this far apart. |
| `crates/mapgen/src/open_country/mod.rs` | `WALK_M` | `50.0` | W | Road is measured, and a settlement's edge sampled, this often. |
| `crates/mapgen/src/open_country/mod.rs` | `PLACED_MAX` | `4096` | W | The most things of one kind a map is given, whatever its rows ask. |
| `crates/mapgen/src/parcels/streets.rs` | `JOIN_OVERSHOOT_M` | `0.5` | I | How far a street runs past the middle of the one it meets, so the two centrelines cross whatever a centimetre of rounding did to either. |
| `crates/mapgen/src/parcels/streets.rs` | `FACING_PAST_M` | `10.0` | P | A street runs to an end that faces it when that end is no more than this far past the first carriageway it would cross. |
| `crates/mapgen/src/parcels/streets.rs` | `PARALLEL_COS` | `0.866` | P | Two carriageways within this of parallel run the same way. |
| `crates/mapgen/src/parcels/streets.rs` | `FACING_COS` | `0.5` | P | Two streets that stop end to end run opposite ways within this (60 degrees). |
| `crates/mapgen/src/parcels/streets.rs` | `SLANT_SIN` | `0.342` | P | A street comes to a carriageway on its own line no farther off square than this (the sine of 20 degrees). |
| `crates/mapgen/src/parcels/streets.rs` | `TURN_SIN` | `0.47` | P | One that would come to it farther off square, up to this (the sine of 28 degrees), turns at its last crossing to meet it square: a bend, where a sharper turn would be a hook. At more of a slant it does not meet it. |
| `crates/mapgen/src/parcels/streets.rs` | `TURN_WIDTHS` | `3.0` | P | A street that turns to meet a carriageway square runs at least this many of its widths from the turn to the carriageway. |
| `crates/mapgen/src/parcels/streets.rs` | `IN_LINE_COS` | `0.906` | P | Two streets that meet a road from its two sides make a crossroads when they run opposite ways within this (25 degrees). |
| `crates/mapgen/src/parcels/streets.rs` | `AHEAD_SPREAD` | `0.36` | P | A street that stops in the open looks this far to either side of its line for the carriageway it stops short of (20 degrees). |
| `crates/mapgen/src/parcels/streets.rs` | `ROAD_EDGE` | `2.0` | P | How much an edge of a district counts for when the district picks the line of its streets: twice its length where a road runs along it, a quarter where no carriageway does. |
| `crates/mapgen/src/parcels/streets.rs` | `OPEN_EDGE` | `0.25` | P | See the owning source for its geometric use. |
| `crates/mapgen/src/parcels/streets.rs` | `ON_WAY_M` | `1.0` | I | A joint this near a carriageway's paving lies on it. |
| `crates/mapgen/src/parcels/streets.rs` | `BOW_LENGTHS` | `[1.2, 2.4]` | P | A bowed street's wave is this many times its district's length, or the preset's wavelength where that is longer: one bend or less from end to end, never a ripple. |
| `crates/mapgen/src/parcels/streets.rs` | `BOW_REACH` | `0.04` | P | And it swings no farther off its line than this share of that length, so a short street bends as gently as a long one. |
| `crates/mapgen/src/parcels/streets.rs` | `ALONE_WIDTHS` | `4.0` | P | A street that is one run from a crossing to the road ahead is at least this many widths long: shorter, it is a connector between two streets that already lie side by side. |
| `crates/mapgen/src/parcels/streets.rs` | `CROWD_WIDTHS` | `3.0` | P | Two streets of one grid come to a carriageway no nearer each other than this many of their widths. |
| `crates/mapgen/src/parcels/streets.rs` | `OWN_TAN` | `0.2` | P | A link runs to a node of its own grid that lies no farther off its line than this share of the way there. |
| `crates/mapgen/src/parcels/streets.rs` | `CORNER_TAN` | `0.47` | P | A street stopped in the open turns to another's end that lies no farther off its line than this share of the way to it (25 degrees). |
| `crates/mapgen/src/parcels/streets.rs` | `CARRIED_M` | `1.0` | I | A district's edge carries a carriageway whose own edge is this near it. |
| `crates/mapgen/src/parcels/streets.rs` | `END_MARGIN_M` | `1.0` | P | A street that ends in the open stops this far inside its district. |
| `crates/mapgen/src/street_props.rs` | `EDGE_M` | `1.0` | P | A body keeps this far inside the map's edge. |
| `crates/mapgen/src/street_props.rs` | `SLACK_M` | `0.05` | I | A body is set this far past the line it must keep to, so rounding its centre to a centimetre cannot put it over. |
| `crates/mapgen/src/street_props.rs` | `DOOR_REACH_M` | `12.0` | P | A door's way to the street is this long where no carriageway lies ahead. |
| `crates/mapgen/src/street_props.rs` | `DOOR_LOOK_M` | `40.0` | P | How far ahead of a door a carriageway is looked for. |
| `crates/mapgen/src/street_props.rs` | `OWNER_STEP_M` | `2.0` | W | A side of a carriageway is asked whose district it is this often. |
| `crates/mapgen/src/street_props.rs` | `SITE_ROOM_M` | `1.5` | P | A fence panel, a cabin and loose stock keep this far inside the fence. |
| `crates/contract/src/curve.rs` | `SAMPLE_SPACING_M` | `2.0` | I | See the owning source for its geometric use. |
| `crates/contract/src/river.rs` | `MIN_WIDTH_CELLS` | `3.0` | I | A river is at least this many height samples wide, or the grid cannot carve its bed: a narrower channel falls between samples. |
| `crates/contract/src/river.rs` | `GRID_GRADE_MARGIN` | `0.9` | I | A surface whose grade is `g` everywhere is drawn by grid triangles no steeper than `g √2` (a triangle's rise along each axis is one sample's). Banks keep this share of the grade that would reach the slope cutoff. |
| `crates/contract/src/forest.rs` | `FOLIAGE_SAMPLE_M` | `1.0` | I | Maximum physical foliage-depth integration step, shared by proofs. |

## Extraction ownership

Initial values are copied exactly; stored trigonometric values are never recomputed from nominal degrees. Distinct town and road meet tolerances remain distinct because their callers make different topology decisions.

| Source symbol | Data destination |
| --- | --- |
| `crates/mapgen/src/joints.rs:MEET_M` | `presets.joints.meet_m` |
| `crates/mapgen/src/joints.rs:NEAR_M` | `presets.joints.near_m` |
| `crates/mapgen/src/joints.rs:THROUGH_COS` | `presets.joints.through_cos` |
| `crates/mapgen/src/joints.rs:TAIL_SPARE_M` | `presets.joints.tail_spare_m` |
| `crates/mapgen/src/joints.rs:CARRY_ROUNDS` | `presets.joints.carry_rounds` |
| `crates/mapgen/src/joints.rs:BESIDE_WIDTHS` | `presets.joints.beside_widths` |
| `crates/mapgen/src/joints.rs:JUNCTION_M` | `presets.joints.junction_m` |
| `crates/mapgen/src/joints.rs:SWING_WIDTHS` | `presets.joints.swing_widths` |
| `crates/mapgen/src/joints.rs:SLANT_COS` | `presets.joints.slant_cos` |
| `crates/mapgen/src/joints.rs:SQUARE_WIDTHS` | `presets.joints.square_widths` |
| `crates/mapgen/src/joints.rs:STUB_WIDTHS` | `presets.joints.stub_widths` |
| `crates/mapgen/src/joints.rs:ALONGSIDE_COS` | `presets.joints.alongside_cos` |
| `crates/mapgen/src/joints.rs:GATE_WIDTHS` | `presets.joints.gate_widths` |
| `crates/mapgen/src/joints.rs:TRIM_MARGIN_M` | `presets.joints.trim_margin_m` |
| `crates/mapgen/src/joints.rs:PIN_ROUNDS` | `presets.joints.pin_rounds` |
| `crates/mapgen/src/joints.rs:SHORTEST_RUN_M` | `presets.joints.shortest_run_m` |
| `crates/mapgen/src/layout/roads.rs:FORK_SIN` | `presets.roads.fork_sin` |
| `crates/mapgen/src/layout/roads.rs:SIDE_ROAD_TRIES` | `presets.roads.side_road_tries` |
| `crates/mapgen/src/layout/towns/ground.rs:SLIVER_M` | `presets.towns.geometry.sliver_m` |
| `crates/mapgen/src/layout/towns/ground.rs:SHARED_M` | `presets.towns.geometry.shared_m` |
| `crates/mapgen/src/layout/towns/ground.rs:MEET_M` | `presets.towns.geometry.meet_m` |
| `crates/mapgen/src/layout/towns/ground.rs:PLACES` | `presets.towns.geometry.places` |
| `crates/mapgen/src/layout/towns/ground.rs:IN_LINE_SIN` | `presets.towns.geometry.in_line_sin` |
| `crates/mapgen/src/layout/towns/streets.rs:SLANT_SIN` | `presets.towns.geometry.avenue_slant_sin` |
| `crates/mapgen/src/open_country/mod.rs:LINE_STEP_M` | `presets.open_country.line_step_m` |
| `crates/mapgen/src/open_country/mod.rs:WALK_M` | `presets.open_country.walk_m` |
| `crates/mapgen/src/open_country/mod.rs:PLACED_MAX` | `presets.open_country.placed_max` |
| `crates/mapgen/src/parcels/streets.rs:FACING_PAST_M` | `presets.parcels.geometry.facing_past_m` |
| `crates/mapgen/src/parcels/streets.rs:PARALLEL_COS` | `presets.parcels.geometry.parallel_cos` |
| `crates/mapgen/src/parcels/streets.rs:FACING_COS` | `presets.parcels.geometry.facing_cos` |
| `crates/mapgen/src/parcels/streets.rs:SLANT_SIN` | `presets.parcels.geometry.slant_sin` |
| `crates/mapgen/src/parcels/streets.rs:TURN_SIN` | `presets.parcels.geometry.turn_sin` |
| `crates/mapgen/src/parcels/streets.rs:TURN_WIDTHS` | `presets.parcels.geometry.turn_widths` |
| `crates/mapgen/src/parcels/streets.rs:IN_LINE_COS` | `presets.parcels.geometry.in_line_cos` |
| `crates/mapgen/src/parcels/streets.rs:AHEAD_SPREAD` | `presets.parcels.geometry.ahead_spread` |
| `crates/mapgen/src/parcels/streets.rs:ROAD_EDGE` | `presets.parcels.geometry.road_edge` |
| `crates/mapgen/src/parcels/streets.rs:OPEN_EDGE` | `presets.parcels.geometry.open_edge` |
| `crates/mapgen/src/parcels/streets.rs:BOW_LENGTHS` | `presets.parcels.geometry.bow_lengths` |
| `crates/mapgen/src/parcels/streets.rs:BOW_REACH` | `presets.parcels.geometry.bow_reach` |
| `crates/mapgen/src/parcels/streets.rs:ALONE_WIDTHS` | `presets.parcels.geometry.alone_widths` |
| `crates/mapgen/src/parcels/streets.rs:CROWD_WIDTHS` | `presets.parcels.geometry.crowd_widths` |
| `crates/mapgen/src/parcels/streets.rs:OWN_TAN` | `presets.parcels.geometry.own_tan` |
| `crates/mapgen/src/parcels/streets.rs:CORNER_TAN` | `presets.parcels.geometry.corner_tan` |
| `crates/mapgen/src/parcels/streets.rs:END_MARGIN_M` | `presets.parcels.geometry.end_margin_m` |
| `crates/mapgen/src/street_props.rs:EDGE_M` | `presets.street_props.edge_m` |
| `crates/mapgen/src/street_props.rs:DOOR_REACH_M` | `presets.street_props.door_reach_m` |
| `crates/mapgen/src/street_props.rs:DOOR_LOOK_M` | `presets.street_props.door_look_m` |
| `crates/mapgen/src/street_props.rs:OWNER_STEP_M` | `presets.street_props.owner_step_m` |
| `crates/mapgen/src/street_props.rs:SITE_ROOM_M` | `presets.street_props.site_room_m` |

Finite advanced iteration/placement counts are bounded at 4096 (the existing maximum country placement count), and sampling steps remain finite positive. These are native-job work safeguards; physical coverage retains its independent proof envelope. Released compile part/bay envelopes remain fixed; editable ground-point allowances are positive u64 work limits.
