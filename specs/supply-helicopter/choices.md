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
