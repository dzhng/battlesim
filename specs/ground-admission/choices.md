# Choices

Decisions an implementing pass made where its slice was silent. One entry each: date,
slice, the decision, the evidence, the alternative it rejected. Planning decisions live in
the [README](README.md#decisions).

## 2026-10-10 — slices 02–03 (top attack)

- **Top-attack rows turn fast (300–360°/s), not at the generic 60°/s.** At 60°/s the turn radius
  (about 190 m) is too wide to pull into the dive cone, so the missile loops or ploughs short
  (`assets/top-attack/sweep-slow-turn.txt`). At 360°/s, loft 60 m and dive 40°, every range from
  100 to 900 m hits the roof at 46° or steeper (`sweep-fast-turn.txt`). Rejected: a stateless
  glide-path aim, which never looped but struck at only 15–30°.
- **`dive_deg` is the minimum impact angle;** real impacts land between it and about twice it.
- **A released missile drops its loft** and goes to ground at its fixed point, as the contract says.
- **Overhead launch needed no launch-check change.** A garrisoned launcher's rounds already pass
  its own building, crowns only block sight and trunks are upright, so the climb clears what the
  straight launch line clears (tests in `garrison.rs`, `guidance.rs`). If a lofted launch check is
  ever added, the loft must join the launch-solve cache key (`solve_launch_past`).
- **Refusal tests live in `top_attack.rs`**, not `flight_load.rs` (that file is the load report).
