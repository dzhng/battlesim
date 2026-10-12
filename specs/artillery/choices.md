# Artillery choices ledger

This is the final decision ledger for the shipped implementation. User-set
decisions live in the feature README; these entries record load-bearing
implementation choices where the plan left the mechanism open.

## Sound

### All twelve physical units share five authored indirect weapon profiles

When the roster rows were enabled, the implementation gave them five shared
weapon rows (`artillery_mortar`, `artillery_howitzer`, two rocket profiles and
one strike-missile profile). Each unit still has its own body, mount, sensors,
appearance and balance data, while weapons carry the common range, minimum
range, arc and scatter rules. The alternative would have been twelve nearly
identical weapon definitions, which would let the profiles drift while adding
no player-facing behavior.

Gap: the plan required every planned unit to be fieldable but did not prescribe
whether weapon data should be shared.

Reach: future artillery balance changes can adjust a profile once; a unit that
needs genuinely different behavior can receive a new authored row.

Verdict: sound. Confidence: high.

### The shared wheeled hull limits were widened to admit the authored roster

Some newly physical artillery vehicles are longer and have a larger turning
radius than the old global wheeled limits. The catalog therefore raises those
limits so the real bodies can load and move; the catalog validator still
rejects any later wheeled body that exceeds the declared limits. The alternative
would be to shorten the source dimensions or leave units disabled, both of
which would make the physical catalog lie about the vehicles.

Gap: the plan required all twelve source vehicles to resolve, but the existing
catalog used global physical limits rather than per-unit exceptions.

Reach: future wheeled assets are judged against the same shared physical
contract, so changing these limits is a balance and collision decision for the
whole roster.

Verdict: sound. Confidence: medium.

### Shipped artillery rows use a shared feasible high-arc timing envelope

The first authored values looked plausible but could not reach their own
declared ranges before their lifetimes expired. The final rows use a common
indirect timing envelope and the global flight lifetime bound admits it; a
production test now loads every shipped row and solves its declared maximum on
the high arc. The alternative would have left a row that parsed and appeared
in the catalog but silently fell back to a low arc or expired before impact.

Gap: the initial slice proved only a synthetic long-range profile, not each
shipped row.

Reach: future artillery rows must pass the same per-row high-arc feasibility
test before they can claim their range.

Verdict: sound. Confidence: high.

### The spread preview is one clamped world ring owned by pointer paint

While Attack Ground is armed, the existing pointer-paint feed draws one red
ring at the cursor. Its radius is the largest selected ground weapon's angular
spread multiplied by cursor distance, clamped to a small readable minimum and a
large map-safe maximum. The alternative would be a permanent unit ring, a
second range visualization or one circle per selected unit, all of which would
teach a different meaning for the same ground mark.

Gap: the request specified distance growth and min/max display size but left the
exact owner and bounds open.

Reach: every armed unit gets the same cue, and changing the display bounds or
colour remains a presentation-only decision.

Verdict: sound. Confidence: high.

### The artillery flight scene installs proxy emitters instead of the full roster art

The focused scene exercises production WASM flight and terrain with proxy
emitters, so it does not install the whole appearance library. The full roster
asset set exceeds this local WebGPU device's 256-layer texture-array limit; the
scene would otherwise fail before its assertions. Proxy emitters keep the
flight contract visible without changing production battle rendering, whose
appearance path remains covered by the asset checks.

Gap: the scene contract required flight evidence but did not require roster
models in the same diagnostic route.

Reach: a device with a larger texture-array limit can add model framing later;
the scene's physics checks do not depend on that presentation resource.

Verdict: sound. Confidence: medium.

### Static artillery source meshes use fixed hull mounts with explicit visual overhang tolerance

The artillery GLBs contain complete static vehicles but do not contain the
renderer’s generic `turret`/`gun`/`muzzle` articulation chain. The physical rows
therefore keep a real muzzle for simulation and mark the mount fixed for asset
fit, while appearance entries allow a long barrel or launcher to overhang the
compact collision hull. A fabricated rig would claim the model can animate when
it cannot; widening the collision hull would make barrels block movement.

Gap: the physical roster slice delegated source-specific fit decisions.

Reach: future artillery art can add a real rig and remove the tolerance without
changing weapon, command or AI data.

Verdict: sound. Confidence: high.

### AI guesses are deterministic, bounded area-biased ground orders

When an artillery unit has no suitable identified target, the policy chooses the
nearest fresh ground contact inside the indirect weapon’s authored minimum and
maximum range. It extrapolates matching observed movement for at most three
seconds, then adds a stable offset inside the contact’s reported area. The
command remains ordinary `Attack` with `TargetRef::Ground`, so the battle
authority still resolves terrain, collision, minimum range and hold-fire. The
alternative would be to read hidden battle state or invent a new artillery
command, either of which would break the observation boundary or duplicate the
command vocabulary.

Gap: the plan delegated evidence scoring, prediction horizon and deterministic
variation.

Reach: changing the guess personality changes one pure policy helper without
widening observations or granting hidden-state access.

Verdict: sound. Confidence: high.

### Attack Ground emphasis is derived from weapon trajectory metadata

The command bar adds an inset accent border only when a selected unit mounts a
weapon whose catalog row says `trajectory: "indirect"`. Mixed selections keep
the existing command availability, shortcut and serialization. A role or unit
ID would drift from the catalog; reading the session weapon rows lets every
future indirect weapon receive the same cue automatically.

Gap: the visual slice delegated the exact existing focus treatment.

Reach: the cue remains a presentation detail on the existing tactical control;
no new command or second range visualization is needed.

Verdict: sound. Confidence: high.

### The artillery lab reuses the production flight bench

The focused artillery route points at the existing ballistics lab and swaps its
synthetic indirect probe for the shipped mortar row. Its ridge-clearance and
minimum-range assertions exercise the same WASM flight owner as the production
weapon; the general ballistics bench continues to own collision and spread
probes. The alternative would be a second artillery-only scene harness with
duplicate physics setup and weaker parity.

Gap: the plan required a focused lab but did not require a new renderer scene.

Reach: future projectile presentation tests have one bench and one source of
flight evidence to maintain.

Verdict: sound. Confidence: medium.
