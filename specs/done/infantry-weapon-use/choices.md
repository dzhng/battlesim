# Implementation choices

These are the final choices made where the request left room for judgment. All are sound; confidence means how sure the auditor is that the user would choose the same tradeoff. Review the medium-confidence choices first.

## Sound — medium confidence

### Useful threats determine window requests

An idle gunner holding fire can still watch a tank through a useful window. Window planning shares normal target selection and range checks, but permits this watching only while there are no known attackers. After an attack, normal return-fire restrictions apply. A fresh out-of-range threat alone cannot turn the gunner toward an unusable window. The alternative was a separate expensive-target ranking rule for windows.

**When:** Native integration, `91dc7730`. **Gap:** The request did not specify idle window orientation. **Reach:** Future window planning shares weapon usefulness; watching grants no firing permission. **Verdict:** Sound, because orientation and firing remain separate. **Confidence:** Medium.

### Torso-bound diagonal carry

When the gunner uses his rifle, his launcher sits diagonally outside his backpack, attached to his torso with two straps. It follows the existing rifle animations rather than gaining a separate equipment simulation. The prompt asked for a launcher on the back without choosing its placement. The silhouette reads in close views; the straps are not reliably readable at tactical distance.

**When:** Carried art, `308eb8d9`. **Gap:** Attachment and silhouette were unspecified. **Reach:** Future rifle poses must preserve equipment clearance. **Verdict:** Sound, using the existing model and animation system. **Confidence:** Medium.

### Keep the facing-facade window allocator

If a gunner already occupies a window facing his target, the allocator keeps him there. It does not search other windows on that same building face for a better trajectory. Prospective weapon assessment lets him request a suitable face without first owning a seat, but the existing allocator still chooses the seat. Adding a trajectory-aware seat planner would widen this feature.

**When:** Garrison integration, `91dc7730`. **Gap:** Exclusivity did not specify new seat optimization. **Reach:** A blocked shot may remain blocked at the current facing window. **Verdict:** Sound as a bounded scope choice, with the limitation explicit. **Confidence:** Medium.

## Sound — high confidence

### Physical aim belongs to each gun

A gunner puts away his rifle to aim his launcher. Only his rifle loses aim; the guards retain theirs. Each physical cycle—the gun's magazine, reload, aim and firing progress—therefore owns aim. The shared targeting lock—the squad's selected target—does not. Keeping shared aim would either grant the returning gunner free aim or reset every guard when he switches. Inactive reload pauses and cooldown—the wait before another shot—elapses; ownership and ammunition remain.

**When:** Native integration, `91dc7730`. **Gap:** The request left aim ownership open. **Reach:** Future switching preserves each physical gun's state. **Verdict:** Sound, keeping the effect local to the soldier who switches. **Confidence:** High.

### Choose before any gun works, and retain useful reloads

Facing a tank, the gunner keeps a useful launcher selected while aiming or reloading. Choosing whichever gun is ready would switch to the rifle every tick and starve the launcher reload. Candidate discovery considers his carried guns independently of yesterday's selection; the simulation chooses one before advancing aim, reload or fire, so the first switching tick cannot work both guns.

Guidance means the gunner controlling an already launched missile. The priority is: guidance → useful assigned single gun → legally usable default rifle → assigned gun needing reload work → idle default gun. “Useful” means it has a legal target and ammunition that can engage it; a default rifle can remain selected and fire even when it cannot damage the target’s armor. A stationary weapon still obeys its existing movement interruption rule. Switching adds no equip timer, while ordinary aim still applies.

**When:** Native integration, `91dc7730`. **Gap:** Ordering and readiness priority were unspecified. **Reach:** New infantry weapons must join the same selection rule; vehicles remain independent. **Verdict:** Sound, preventing both simultaneous work and reload starvation. **Confidence:** High.

### Whole-body active and carried sets

The same soldier changes from a launcher-held model to a rifle-held model with the launcher on his back. Both sets keep his variant identity and supply their own clips and muzzle socket. Runtime attachments would introduce a second equipment owner beside the existing whole-body art. The authored `operator_appearance` becomes an object with equal nonempty `active` and `carried` sets; each set shares its own skeleton, while the two holds may use different clip families. There is no old-array compatibility parser.

**When:** Art and presentation integration, `308eb8d9` and `91dc7730`. **Gap:** The asset structure was open. **Reach:** New carrier art supplies both holds and migrates authored data together. **Verdict:** Sound, extending the existing art owner. **Confidence:** High.

### Visible identity carries authoritative activity

When the launcher runs dry and the gunner selects his rifle, the browser receives that choice directly. Each visible member publishes an authored mount index—a gun’s position in the unit’s weapon list—or null beside ID and slot. Packed identity rows gain a fourth value, with `-1` representing null. The mount's operator separately identifies who carries it. Enemy filtering applies to member activity and operator identity; presentation never guesses selection from timers or old shots. Selection and physical aim enter deterministic digests.

**When:** Publication integration, `91dc7730`. **Gap:** Transport was unspecified. **Reach:** Every observation consumer uses the same member identity/activity contract. **Verdict:** Sound, keeping simulation authority and visibility intact. **Confidence:** High.

### Each soldier requests one weapon's window

With a tank north and infantry south, one gunner cannot request both windows for launcher and rifle. Window requests reuse weapon priority at prospective seats. That assessment lets him request a window he does not yet own; actual firing still requires the occupied window and normal permission. Priority follows authored single-gun ownership, rather than the number of survivors: a last surviving ordinary rifleman must not become a specialist merely because he is alone.

**When:** Garrison integration, `91dc7730`. **Gap:** Concurrent window requests were unspecified. **Reach:** New weapons share one request decision per soldier. **Verdict:** Sound, avoiding contradictory positioning requests. **Confidence:** High.

### Delayed launches remain attached to their weapon

A rifle fires on one tick, the soldier switches to his launcher, and the earlier rifle launch appears in the next publication. Per-soldier counters retain the mount identity, so it cannot trigger a launcher firing pose. A lingering flash from a gun no longer held uses the published launch point instead of another barrel's socket. Conversely, acquiring a carried launcher while continuing rifle fire preserves that rifle's valid animation evidence.

**When:** Presentation integration, `91dc7730`. **Gap:** Delayed visual evidence was unspecified. **Reach:** New weapon switches must retain launch identity independently of current equipment. **Verdict:** Sound, preventing stale evidence from posing or flashing the wrong gun. **Confidence:** High.

### Equipment changes install the new pose directly

When a soldier switches holds, blending the old launcher skeleton's pose into the rifle skeleton can bend unrelated bones. Equipment-family changes therefore clear incompatible blending and install the new pose directly. Posture changes within one hold still blend.

**When:** Pose integration, `91dc7730`. **Gap:** Cross-equipment blending was open. **Reach:** New holds must resolve model, clips and muzzle together. **Verdict:** Sound, preserving valid pose families. **Confidence:** High.

### Fallen carriers always use ordinary kit

When a gunner dies and a survivor recovers the launcher, drawing it on both soldiers would duplicate one weapon. Every fallen carrier therefore uses ordinary kit, including when no survivor recovers the launcher. The cost is that corpses do not visually retain special equipment. The unused carried model's death endpoint intersects the ground; living clips and ordinary gameplay deaths are the accepted contract.

**When:** Death/presentation integration, `91dc7730`. **Gap:** Fallen equipment was unspecified. **Reach:** Death clears carrier and active appearance overrides universally. **Verdict:** Sound as a simple casualty policy with its visual cost disclosed. **Confidence:** High.

### Clear fine routes preserve the search result

On open ground, a soldier's short route can bypass A*—the grid search—only when a conservative rectangle proves every relevant ground cell, obstacle and standing soldier clear. Otherwise it uses the existing search. Checking only the straight line could miss diagonal cells or a different result near an endpoint. The result reports whether a search occurred, so the existing work counter measures searches rather than proven connectors. The requirement is the same waypoint result and battle state, not merely reaching the same goal.

**When:** Closeout, `17983aa5`. **Gap:** A pre-existing search-bound failure needed a behavior-preserving resolution. **Reach:** Future shortcuts must retain the fallback and route semantics. **Verdict:** Sound; conservative proof supplies the general rule, while sampled parity checks supply bounded evidence. **Confidence:** High.

### Scripted rejoin destinations are spread per unit

Two healed squads sent to the same village-center destination can compete for one occupied settlement; one correctly refuses it. The comparison controller uses its existing per-unit spreading rule when sending squads back, as it already does around supply. Raising movement budgets or permitting overlapping destinations would change the game to satisfy the report.

**When:** Closeout, `17983aa5`. **Gap:** The failed comparison setup was not specified by this feature. **Reach:** Future before/after reports must use the same controller on both arms. **Verdict:** Sound, repairing requested destinations rather than admission rules. **Confidence:** High.

### Comparisons control unrelated variables

A requested movement command does not prove a fight started: scenes now require admitted ordinary destinations and advance through their application ticks. Building presence compares the same projected roof pixels with buildings shown and hidden, because a grey roof can resemble an adjacent grey road. Road material comparisons use the same illumination, excluding unequal building shadows. The physical-flight bench stages its direct crest shot against the current flight and follows the actual crossing-impact tick. Its height check projects the launched arc rather than a fixed expected height. The distant-bridge fixture starts facing along its narrow road, retaining its original arrival deadline, clearance, turning-radius and water assertions; an unrelated initial cross-road turn had spent the road’s speed advantage. Each check retains its intended physical or visible requirement.

**When:** Closeout fixture repairs. **Gap:** Existing staging and measurement assumptions failed independently of infantry selection. **Reach:** Future harness work must prove its setup and hold unrelated variables fixed. **Verdict:** Sound, testing the contract without retuning gameplay or weakening thresholds. **Confidence:** High.
