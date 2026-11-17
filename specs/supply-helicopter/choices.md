# Supply helicopter: choices ledger

Choices made where the spec was silent, appended per pass. Each is consolidated
against the shipped code at closeout (slice 11).

## Slice 01

| Choice | Verdict |
|---|---|
| "Drives" means the type has ground locomotion (`mobility.locomotion().is_some()`), the one predicate in `skirmish_ai/purchase.rs`. | Sound: it reads the physical property, and every ground mover has locomotion. |
| The drives filter sits inside `pick` for the `logistics` role, so the opening recipes and the truck replacement both get it without each caller filtering. | Sound for now. The role is the doctrine's own word for its standing supplier. Slice 09 selects the flying supplier with its own filter and must not route through `pick("logistics")`. |
| The "already have a supplier" check counts only owned or pending suppliers that drive, so an emergency heli never stands in for a lost truck. | Sound; matches D11. |
| Tests patch `test_heli` inside the test deck into a flying supplier instead of adding a fixture row. | Sound; the slice asked for an in-test patch until slice 02's row exists. |

## Slice 02

| Choice | Verdict |
|---|---|
| `drop_duration` is stored on the deployment's tick lattice (drop ticks × deploy step), so it compares directly with `current`. Codex first stored it in plain ticks, which mismatched the lattice; fixed in integration. | Sound; one unit for progress. |
| The landed latch is set on the tick progress reaches the drop's end while deploying, and cleared at progress 0 while packing. Codex first set it at full deployment; fixed in integration to match Q2 (ground vehicle from touchdown). | Sound; pinned by `its_height_falls_with_setup_and_it_touches_down_before_it_serves`. |
| An aircraft "lands" when its deployment has a drop (`drop_duration > 0`); no separate flag. | Sound; one source of truth. |
| The height law starts from the height held when the deploy timer first moved (`landing_start_z`, folded into the unit digest while set) and eases out quadratically to the ground. | Sound; the start height is state that decides the future, so it is digested. |
| While packed with a movement goal and below the low hover, it climbs straight up before moving sideways (P10). | Sound. |
| Adding `test_supply_heli` renumbers the test catalog's type indices. Three paired-record fixtures (`scenario-overrides`, `publication/stream`, `publication/contact-lifecycle`) were re-blessed. Their digests first diverge at the first sighting, because contacts fold the type index. Codex had instead added a filter that dropped the new row from those tests' catalogs, which is a compatibility shim; it was removed. | Sound; a named re-bless, no outcome change. |
| The lab lists each supplier by its type's name, not by a kind-id string. | Sound; no presentation keyed on an id. |
| The heli variant's camera frames the landing point and the recipients at 150 m. | Reversible; judged from the scene frames. |
