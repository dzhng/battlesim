# 05d Lawns and yards dressed; vehicles need a way in, not a way everywhere

**Why:** after 05c the structure reads (paved yards, lawns) but lawns are sparse and centre districts unwalled (`../assets/05c/`). Both come from a stricter rule than the plan's decision 5: lanes kept a tank-wide ring round every group and a tank's way beside every wall. Decision 5 asks for a way in, for infantry and for vehicles, into each court. Dense courtyards being mostly infantry ground is what a viewer expects and what makes a town fight.

**Unlocks:** courts that read as lived in: walled yards in every dense district, lawns crossed by paths and dressed.

**Seam:** `street_props/courts.rs`, `parcels/courts.rs`, presets; the reachability tests' claims (the one place the rule is stated).
- The vehicle claim becomes: every court (yard, car park, lawn) keeps at least one way in from a street for the widest hull, and every door's way out stays clear. The infantry claim stays: squads reach every yard gate, door, car park and lawn point they reached before.
- Lawn and yard groups keep a squad-width ring, not a hull-wide one; a lawn keeps one hull-wide lane from a street through it.
- Yards are bounded wherever a squad passes between boundary and building, with squad-wide gates by each door and one hull-wide gate per yard where its side allows.
- Paths: paved strips across each lawn joining its gates and the streets (a `Paving` polygon per path), planted beside.
- Lawn dressing from existing kinds with art: trees, hedges, benches, bins, playgrounds, the region's signature groups.

**Verify (tests first):** the reworded reachability claims (vehicle way into each court, squads everywhere as before, doors clear); 05c's structure metric no worse; a lawn fill measure (share of lawn farther than 8 m from any body, path, building or paving) held low per region from evidence; sweep 81/81; part limit; determinism; the move probe no worse than the pre-courtyards baseline (38 maps; current branch 31 after the cover fix and 05c's change — re-measure); parity re-bless. Shots per region (the 05c framings, plus one apartment and one core district court so walls show), screenshot-critique, compare-screenshots against `../assets/05c/`.

**Delegated:** path layout, ring widths within squad width, lawn group mix per region.
