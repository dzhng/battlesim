# 00 Probes

**Unlocks:** confidence that the ladder stands. Each probe asks one question,
has a kill criterion and a fallback, and costs about an hour. No product code
lands, except P5's bug fix if the bug is real. Write each verdict to
`throwaway/tutorial-probes/<probe>.md` and add one line to
[decisions.md](../decisions.md).

| Probe | Question | How | Kills | Fallback |
|---|---|---|---|---|
| P1 | Can the generator make the valley? | **Generated maps on `staging` are flat today.** Hills are being added on branch `t3/map-gen-rolling-hills`, in a separate session ("Map Gen Hills Status and Spec"). That session is teaching the map's ground-sight check to count a hill crest as cover, so hills can grow past their current 8 m cap. Check whether it has landed on `staging` first, and ask that session if it hasn't. Then, in the map workbench (`apps/map-workbench`), search map type, size and seed for a south road in, a wooded ridge, a farm with a garrisonable house and a wall, a crossroads settlement, a river with a bridge, and a northern hill tall enough to hide a radar from the village. Save the best seed's top-down plan picture. | "Generated once" | If hills haven't landed, or stay too gentle to read as a ridge and a hill: save the closest generated valley and author its relief as an `authored` saved map (the sim reads authored hills the same as generated ones). Otherwise accept the closest geography and adapt the zones ([story.md](../story.md) binds to zones); a ford may stand in for the bridge. Don't block on the hills branch. |
| P2 | Is restart by replay bearable late in the tutorial? | Record a ~25-minute small offline skirmish (~30 units a side). Measure replay cost to its end: instructions per tick natively (`crates/sim/examples/replay_report.rs` or similar), and the WASM fast-forward through a worker with publications (`SimClient.advance`), because browser publication overhead counts too. | Restart by plain replay | Show replay progress on the loading screen. If chapter 7 would take longer than about a minute, reslice 12: keep replay as the proof and hold the replay at each chapter start in the worker (still replay, no clone). |
| P3 | Do pre-placed, player-owned units work inside a skirmish? | Native test on test units: a `ScenarioDefinition` with a skirmish and authored owned `units`. Check the unit cap counts them, kills pay bounty and settle from some price, and refunds behave. | Forces as authored `units` | If settlement needs a receipt, give authored units a zero-price receipt at setup (decision recorded). |
| P4 | Does the roster cover every story role? | Read the resolved catalog. For each role in [story.md](../story.md) (recon, squad with carrier, AT team, tank, supply truck, mortar, air defence, helicopters, jet, SEAD jet, radar-emitting SAM), find a `us` or `eastern` card with the needed capability. Record the pick per role. | The story as written | Swap the unit in the story, never the rule. If a spoken line names a role that no longer fits, re-render that line (slice 10). |
| P5 | Are a player-owned authored script's orders refused as out of sequence? | `run_authored_scripts` stamps `seq = next_seq[side]` (`crates/sim/src/battle/scripts.rs`), but a command naming a player is checked against the player's own counter (`accept_order.rs::expected_seq`), which is what the skirmish AI uses (`skirmish_turn.rs`). Write a sim test where a player-owned script issues two orders, and check the second is accepted. | Nothing in this feature (the director uses the fixed-script path) | If it's a real bug, fix it in its own commit, with the test, before anything else lands. |

**Gates:** each probe's verdict is written down. P5's test passes, either
unchanged (no bug) or with its fix.

**Delegated:** the probe harnesses' shape. They live in `throwaway/`, except P5's
test, which stays in the sim crate if it found a bug.

**Feedback that changes it:** a P1 picture the user dislikes sends the search
on; a P2 result far over budget reslices 12 before 08.
