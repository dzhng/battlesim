# 04 — Active/passive controllers and deterministic takeover

## Contract unlocked

Every non-human slot runs an observation-bound controller, and switching a human slot to active AI changes control deterministically without creating a second battle or hidden truth path.

## Seam

Create one simulation controller host. Each controller receives only the observation available to its slot's combat side and emits ordinary validated commands. Reuse the existing faction-aware `skirmish_ai`/opponent policy seams rather than creating a second AI authority.

- Active AI adapts doctrine to faction, visible threats, terrain, allies, and objectives.
- Passive AI holds position and may return fire/self-defend when threatened.
- Neutral policy never initiates combat.
- Controller switch is a simulation-timed state transition; replay consumes recorded accepted commands and does not rerun live controllers.

## Runnable artifact

A deterministic native/WASM battle runs one human slot, one active AI slot, one empty/passive slot, and one neutral slot. The human slot switches to AI at a known tick; the replay reproduces the same accepted command stream and outcome.

## Verification

- metamorphic hidden-state tests prove controllers cannot target unseen truth;
- passive AI does not issue unrelated autonomous orders;
- return-fire behavior is deterministic;
- takeover and controller ordering are replay-stable;
- command-rate and work bounds are explicit and measured.

## Firewalls

No network ownership, rollback, concurrent human session, or controller-specific hidden map access. Scripts do not run in this slice; slice 05 freezes their host first.

