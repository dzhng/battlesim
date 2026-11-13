# Smoke screen: choices

Choices made where the spec was silent, appended each pass (`audit-choices`). The
user's decisions are in [`decisions.md`](decisions.md). The **Pass** column names
the slice or pass that made the choice.

| Pass | Choice | Verdict | Why |
|---|---|---|---|
| write-spec | A cloud's law moves in steps in **both** the simulation and the GPU fog. Bloom is 4 steps over `bloom_s`. Fade is 5 steps over `fade_s`, scaling depth per metre and the concealment shortfall `1 − concealment` together. | Settled | One stepped law that the simulation and the screen share, so they never disagree. It also caps GPU eye rebuilds. Proposed by the seam draft. |
| write-spec | Overlapping clouds add their chords together, so the overlap is denser. | Settled | D1 taken literally; a viewer would expect it. |
| write-spec | A side sees a cloud when its simulation fog field marks any cell whose centre lies within `radius + fog_cell_m` of the cloud's centre. Its own clouds are always seen. | Settled | A cloud hides its own centre, so only its near rim can be seen. This reuses the sweep as the single owner. The rule came from the fewest-slices and seam drafts. |
| write-spec | Smoke reaches the GPU fog through the publication, inside the presentation crate (`presentation/side_fog.rs`), not through the TS or native foliage builders. | To confirm in slice 02 | Per the seam draft, eyes already arrive this way. The risk-first draft assumed both builders change; slice 02 settles it. |
| write-spec | Wire- and laser-guided missiles losing guidance needs wiring: `guidance_clear` (`weapons.rs:914`) and the guided return-fire check (`weapons.rs:1008`) move to `sight_depth`. | Settled | The map called this free; two drafts found both read `foliage_depth` directly. |
| write-spec | The observation field is `OwnUnit.smoke`, never `smoking`. | Settled | `smoking` is already damage smoke (D34, `observation.rs:540`). |
| write-spec | The smoke-capable test rows (`test_smoke_tank`, `test_smoke_squad`) are new rows extending `test_tank` and the test squad. Existing test units stay without smoke. | Settled | Keeps pinned digests unchanged (D23) and gives the lab and tests their carriers. If a catalog identity pin moves, name it here. |
| write-spec | The AI's starting tuning: a unit is "losing" when its hp (or squad `member_hp` sum) has dropped by at least `smoke_loss_share` = 0.2 of its maximum within `smoke_window_s` = 10. It reverses `smoke_reverse_m` = 40. | Settled as starting data | 40 m clears a 15 m cloud with room to spare. The numbers are data, tuned in the balance pass, not targets. |
| write-spec | Three drafts were made (fewest-slices, Claude; risk-first, Codex gpt-6.1-sol; seam-quality, Claude). The ladder takes the risk-first draft's kill gate (02), the seam draft's owners and parity probe, and the fewest-slices draft's dormant-first proof (03–08 keep digests). | Settled | All three agreed on reusing the digest slot, keying the pop sound on its tick (no new cue), and the single layout change. |
