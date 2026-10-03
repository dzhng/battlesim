# Implementation choices

All entries describe the final implementation and are judged sound. Medium-confidence entries are the first decisions to revisit if the intended behavior changes. Core choices landed with the native/presentation integration and carried art; closeout added the navigation and comparison-controller choices below.

## Sound — medium confidence

### Useful threats determine window requests

Window planning shares normal target selection, including range and return-fire permission, while evaluating prospective windows. Idle troops with no known attackers can watch legal threats while holding fire; after an attack, normal return-fire restrictions apply. With no retained lock or guidance, an enemy outside weapon range no longer creates a window request solely because it is expensive. For example, a distant tank no longer turns an idle rifleman toward an unusable window. The alternative was to keep a separate expensive-target ranking rule. This removes a second ranking policy and makes window orientation follow weapon usefulness. It changes idle orientation, not sight from unoccupied windows.

### Torso-bound diagonal carry

The stowed launcher reuses the existing tube and sight, attached diagonally outside the backpack with two retention straps. It follows the body through rifle animations. Its silhouette reads as carried equipment in close views; straps are not readable at tactical distance. The prompt specified a launcher on the back but left its placement open. This cosmetic choice lets the existing rifle animations carry the equipment, and future art must retain that clearance.

## Sound — high confidence

### Physical aim belongs to each gun

Putting away one operator’s rifle abandons only his aim. Guards keep theirs. Each physical cycle therefore owns aim, magazine and reload; the shared lock—the squad’s selected target—owns targeting. Switching keeps ownership and ammunition, pauses inactive reload work and allows cooldown to elapse. Ordinary aim applies when the gun comes back into use.

### Useful weapons keep priority through reload

Guidance reserves the operator first. Otherwise an assigned single weapon with a legal effective target remains selected while aiming or reloading; the rifle is the fallback. For a gunner facing a tank, choosing whichever gun is ready would put away the launcher repeatedly and starve its reload. With no useful target, idle selection permits reload work. Stable mount order breaks ties; existing recovered-spare assignment remains unchanged.

### Whole-body active and carried sets

The existing art combines soldier, equipment, clips and sockets. Paired whole-body sets extend that system without introducing modular runtime attachments. The prompt left the asset structure open. The same variant index keeps a carrier recognizable across holds. The authored active/carried object is a hard cutover; both sets must be nonempty, equal-sized and internally share a skeleton, while the two holds may use different clip families.

### Visible identity carries authoritative activity

Each visible member publishes his active mount alongside ID and slot. The mount’s operator separately identifies who owns the physical weapon. When a launcher runs dry and its operator selects his rifle, the browser receives that choice directly. Presentation does not infer use from timers or old squad shot counts, and enemy filtering applies to activity exactly as it does to member identity. Selection and physical aim enter deterministic state digests.

### Each soldier requests one weapon’s window

Window requests reuse weapon priority and physical assessment. If a tank is north and infantry is south, one gunner cannot request both windows for rifle and launcher simultaneously. Prospective assessment lets him request a window he does not yet occupy; it grants no shooting permission. Authored squad/single ownership determines displacement priority even when only one rifleman survives. Allocation continues to choose a facing facade, without a new search for alternate trajectories on the same facade.

### Launch evidence remains attached to its weapon

The prompt left delayed visual evidence unspecified. Flight first shows a shot on the following tick. Per-soldier launch counters retain mount identity so an old rifle launch cannot animate the newly held launcher. An unchanged rifle’s valid shot survives carrier assignment changes. A lingering flash whose weapon is no longer held uses its published launch point rather than another barrel’s socket.

### Equipment changes reset blending; fallen kit stays ordinary

Model, animation family and muzzle resolve the same active/carried fact. Equipment changes install the new pose directly, including on death; posture changes within one hold still blend. The prompt left fallen equipment open. Fallen carriers use ordinary kit because the recovered launcher already belongs to a survivor. The carried bundle’s standalone death endpoint intersects the ground and is not a gameplay pose; living clips and ordinary gameplay deaths are the accepted contract.

### Clear fine routes preserve the search result

Closeout found repeated fine-grid searches on open ground. A fine route is the short walking path from a soldier to his next destination. It can bypass A*—the grid search—only when a conservative rectangle proves every relevant ground cell and physical obstacle clear. Otherwise it uses the existing search. Checking only the straight line could miss blocked diagonal cells or a different result near an endpoint. The result reports whether a search actually occurred, so the existing work counter keeps measuring searches. This preserves routes and battle digests while reducing avoidable work; future shortcuts must preserve the same result, not merely reach the same goal.

### Scripted rejoin destinations are spread per unit

Closeout found two healed squads being sent to the same village-center destination. One correctly refused the already occupied settlement. The comparison controller now uses its existing per-unit spreading rule when sending squads back, as it already does around supply. The task left this failed comparison setup open; raising movement budgets or permitting overlapping destinations would change the game to satisfy the report. This choice changes the comparison script’s movement requests, so future before/after reports must use the same controller on both arms.
