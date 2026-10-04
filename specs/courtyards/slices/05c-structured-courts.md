# 05c Courts are structured: walled yards, parking and planted lawns

**Why:** with 05b's fill and the real art merged, even the fullest court in each region still reads as an empty plaza (`../assets/05b-art/`): paving a whole block interior as one floor leaves open ground no prop density fills. Real town interiors are divided: each building has its own yard behind a wall, railing or fence; parking lots hold rows of cars; planted lawns with trees lie between. The user's brief was "stone or concrete blocks with actual stuff in them".

**Unlocks:** courts that read as lived-in structure from the town camera (120 m) as well as up close.

**Seam:**
- `parcels/courts.rs` (court geometry, one owner): each built lot of a dense district is paved as its yard (the lot ring, already the parcel), and the district's remaining interior is cut into **parking** (paved rectangles along a carriageway or a yard's gate side, presets-sized) and **lawn** (left as grass: the ground under no paving polygon). Courts stay `Paving` polygons; lawns are the absence of paving, so no new surface kind.
- `street_props` (placement, one legality check): yard boundaries through the one fence routine with a gate on the building's door side, one boundary kind per region from presets (China masonry wall, New York chain-link or iron railing, Paris iron railing on a low plinth; new catalog kinds decided with tweak-mechanics, stand-in art until their art sub-slice); parking lots filled with car rows (existing `parked_car`) leaving a drive aisle; lawns planted with trees, hedges and benches; the slice 05 amenity groups placed in yards and on lawns rather than one plaza.
- Presets: per dense district `props.courts` gains the yard boundary, parking and lawn rows; validated.

**Verify (tests first):** every dense lot's yard is paved and bounded with a gate on its door side; parking lots keep an aisle a car drives; reachability (squads and the widest hull to every yard gate, door, parking lot and lawn), doors clear, determinism, part limit, sweep 81/81 all hold; a structure metric replacing 05b's weak one: the share of dense-court ground that is open paving farther than 6 m from any body, building, boundary, lawn or carriageway, held low per region. Then the move-destination probe stays no worse than the placement lane leaves it.

**Run/see:** the 05b-art framings per region (fullest and median court, town-120-low); screenshot-critique; compare-screenshots against `../assets/05b-art/`.

**Then art:** a sub-slice for the boundary pieces (wall, railing, chain-link reuse) in the 08 pattern, under the 1 MiB cap.

**Delegated:** parking lot sizes and where they go, lawn share, which amenity groups favour yards or lawns.
