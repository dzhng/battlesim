# Final implementation decisions

The user's fixed-band rule, grenade exception, live one-attempt misses and
layer-aware explosions are requirements. This ledger covers the architectural
choices left to implementation, checked against the finished code. All retained
choices are sound; none remains provisional or needs a user decision. Review the
planned-loft cone first: it has medium confidence. The others have high confidence.

## Sound

### Follow the planned aim when checking a loft's tracking cone

- **When:** one-attempt guidance pass, `45de21aee`.
- **Decision:** when a top-attack missile climbs over a tank, its tracking cone
  (the angle it can follow in front of its flight direction) checks the planned
  climb point. When the path dives, it checks the tank aim. A direct missile
  checks its target aim. Checking the tank under a climbing missile instead
  could immediately abandon a valid authored attack.
- **Gap:** the requirements preserved top attack and required a tracking cone,
  but did not define how they combine during the climb.
- **Reach:** future loft authoring inherits this interpretation of the existing
  seeker angle. An independently moving seeker head would need its own policy.
- **Verdict:** sound; steering and tracking use one planned path without a new
  gimbal setting or an exemption for a named weapon.
- **Confidence:** medium.

### Reuse the existing release coast and descent for lost seekers

- **When:** one-attempt guidance pass, `45de21aee`.
- **Decision:** when a seeker loses its target, it takes the same fixed coast
  and descent as a supported missile whose operator loses sight. After passing
  that point it stops steering; ordinary gravity, motor, collisions and lifetime
  continue. A nearby hull can still be hit. A separate miss law would add
  another set of speed, timeout or descent settings.
- **Gap:** a live miss was required, but its exact descent was unspecified.
- **Reach:** misses reuse the existing release tuning rather than growing
  another flight profile.
- **Verdict:** sound; preserves the repository's physical release behavior.
- **Confidence:** high.

### Express finality through existing guidance and target ownership

- **When:** one-attempt guidance pass, `45de21aee`.
- **Decision:** passing the aim removes the flight's steering command; losing a
  seeker target removes that target ownership. The physical round stays alive.
  Before the next flare pass, ended attempts release their launcher support and
  target ownership so they cannot provoke countermeasures as active tracks.
  A separate lifecycle enum would duplicate facts these owners already hold.
- **Gap:** the requirements did not prescribe the state shape or the ordering
  between ended guidance and countermeasures.
- **Reach:** consumers must distinguish a live physical round from an active
  homing attempt. Existing digests already cover the retained owners.
- **Verdict:** sound; one owner per remaining capability, with no new persistent
  state or digest field.
- **Confidence:** high.

### Retain the band on transient paths and bursts

- **When:** visibility pass, `23ff2aebd`.
- **Decision:** a missile may hit and disappear within a tick before a side's
  observation is published. Its path and explosion therefore retain its band
  with the event data. Looking up the removed round later would lose the very
  evidence needed to show that impact.
- **Gap:** the visible result was specified, but the lifetime of its evidence
  was not.
- **Reach:** the simulation grants visibility before publication. Clients need
  no new observation or wire fields, and the extra data is rebuilt each tick.
- **Verdict:** sound; derived evidence survives its source long enough to be
  published without becoming a second battle authority.
- **Confidence:** high.

### Share damage's physical blast origin with airborne impact visibility

- **When:** visibility pass, `23ff2aebd`.
- **Decision:** a terrain collision can round just beneath the surface. A ray
  aimed there sees terrain in its way and hides an otherwise visible explosion.
  Airborne impact sight now samples damage's existing origin just outside the
  struck surface, while the observation retains the actual impact coordinate.
  A new sensor exception or separate height offset would duplicate that rule.
- **Gap:** the requirements did not define sight sampling at a collision surface.
- **Reach:** damage and visibility share one surface convention. Ground fog and
  aircraft-crash bursts keep their existing behavior.
- **Verdict:** sound; reuses the physical origin rather than adding a visibility
  threshold.
- **Confidence:** high.

### Split oversized authority and flight files by their responsibilities

- **When:** preparation and flight-ownership passes, `4752903b2`, `048371686`.
- **Decision:** publishing an observation, admitting a command and handling an
  air lifecycle remain parts of the same battle authority but have readable
  modules of their own. Flight configuration is separate from per-tick swept
  motion and impacts. The public authority, flight store and tick order stay in
  their existing owners. Keeping the oversized files whole would obstruct the
  repository's whole-file review budget.
- **Gap:** the plan required editable files but left the ownership seams open.
- **Reach:** future changes follow a responsibility into one module, without a
  parallel API, compatibility wrapper or new authority.
- **Verdict:** sound; the moves preserve behavior and public import boundaries.
- **Confidence:** high.

### Give the side-contact regression a valid initial flight direction

- **When:** one-attempt guidance pass, `45de21aee`.
- **Decision:** blue and red missiles in the contact-isolation fixture launch
  toward their own contacts, initially aiming short. Guidance must renew those
  aims from the correct side's knowledge. The old setup launched upward with
  ground contacts behind it, so cone enforcement correctly ended its attempts.
  Allowing that artificial setup in production would weaken the new rule.
- **Gap:** the older fixture assumed tracking without a directional constraint.
- **Reach:** contact isolation remains covered while the harness obeys the forward-tracking
  contract.
- **Verdict:** sound; repairs the fixture without a production exception.
- **Confidence:** high.
