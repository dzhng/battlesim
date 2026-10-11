# 01 — Balanced hierarchical doctrine

This slice makes one policy behave like a commander with a force-wide plan.

## Doctrine state

Implement the smallest useful hierarchy:

- **Doctrine state:** opening, recon screen, probe, assault, consolidate,
  emergency defence, recover; each has entry evidence, minimum dwell, maximum
  dwell, and cooldown.
- **Mission plan:** role quotas and task groups for recon, assault, hold,
  anti-armour, air denial, logistics, and recovery.
- **Order emitter:** ordinary purchase, move, attack-move, attack, deployment,
  garrison, stop, and refund orders where legal.

Transitions use only current side-scoped observations: objectives, contested and
capture progress, own survival/suppression/service, visible enemy kinds and
contacts, route availability, and credits. Typed contacts expire normally.

The authored graph is allowed to collapse to a simpler state machine if tests or
tournament evidence show that the graph adds ceremony without better decisions.
The observable contracts and doctrine trace remain unchanged.

## Procurement

Score eligible units from physical data: weapon target layers, effective damage,
range, sensors, mobility, cost, finite ammunition, objective eligibility, and
current force deficits. Use greedy marginal coverage under budget/cap constraints
with minimum recon, infantry, anti-armour, and logistics coverage. A discovered
helicopter raises `low_air` coverage demand; the policy prefers an available
HMG/autocannon/other legal low-air counter, while acknowledging that dedicated
air-defence cards remain disabled.

Randomness may select a near-equal candidate or delay the counter by one decision
window under a bounded miss budget. It cannot choose an incapable unit when the
doctrine requires a hard capability and an affordable capable option exists,
unless the configured bounded-mistake rule explicitly permits that miss.

## Verification

- Tests prove each transition, minimum/max dwell, pivot cooldown, and no rapid
  oscillation.
- A typed helicopter contact changes procurement toward low-air coverage.
- A ground-only match does not pay an unnecessary permanent air-defence tax.
- Tank swarm and infantry pressure buy their intended force shapes while
  respecting economy and cap.
- Repeated losses can pivot from tank swarm to air denial or infantry pressure.
- Hidden enemy state does not affect doctrine state or purchases.

