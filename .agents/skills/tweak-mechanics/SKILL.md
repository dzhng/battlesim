---
name: tweak-mechanics
description: Change or add a game mechanic or rule — who can shoot, see, move, block, push, destroy, take cover, or be hit — by imagining the battle as a war film (Hollywood realism) and making units behave the way a viewer expects. Use before proposing or implementing any rule change, a new body/prop/unit property, a fix for "units behave wrong", or a change that came out of a balance finding.
---

# Tweak Mechanics

**Imagine the battle** as a war film: aim for Hollywood realism, not a ballistics table. The test of any rule is whether units do what a viewer expects a crew or soldier to do in that exact moment, and whether it looks right on screen. Picture the scene as footage, not as a table. If a unit does something an audience would call ridiculous, the rule is wrong, however consistent or physically accurate it looks on paper. If an exaggeration reads better on screen and plays well, like cover blown apart or shells felling trees, keep it. A rule also applies to every kind it touches and composes with every rule on the same path, so find the ridiculous moments yourself, before the user does.

## Workflow

1. **Confirm the cause in the code.** Reports paraphrase, and they can be wrong about the mechanism. Read the rule that actually decided the behaviour, and state it in one sentence: "X happens because rule R returns Y when Z". **Tell:** a fix proposed for a mechanism nobody has read.
2. **List who acts and what they act on.** Actors are every weapon, unit and order type the rule touches. Subjects are every prop, body and unit kind with the property the rule reads. Each subject plays two roles:
   - **as the target:** the thing the actor meant to act on;
   - **in the way:** standing between the actor and something else, which it may hide (an unseen enemy, a friendly unit, the objective).
3. **Picture the battle for each pairing that matters,** from both sides. Narrate the moment: where the unit is, what its crew can see, what they want. Then ask four things:
   - **Would a crew in a war film do this?** Would they hold fire, fire, drive, stop, or wait here? Would they choose a different, obvious option, like driving round the corner, flanking, or waiting for the smoke to clear?
   - **Follow it to the end state.** What does the world look like after the action plays out? A collapsed house is still a ruin in the way, and a cleared lane stays cleared.
   - **Is the data believable on screen?** For each property value the rule reads, ask the viewer's question. Would this stop a bullet? Could you see through it? Would a truck push it? A false value multiplies through every rule that reads it.
   - **What does the other side see and exploit?**
   
   **Done when** every pairing that matters has a narrated moment and a verdict. "Nothing odd" needs a one-line reason.
4. **Walk the neighbouring rules.** For each other rule on the same path (sight and occlusion, cover and concealment, blocking, destruction, targeting, orders against automatic behaviour, knowledge and fog), check three things:
   - **double counting:** two mechanisms modelling one real effect;
   - **freezes:** a unit waiting forever on something that never clears;
   - **degenerate play:** a cheap trick the rule rewards.
5. **Keep it first-principled, and pragmatic.** Rules read bodies, footprints, weight, what can be seen past, and what a round or vehicle can break. If the principled version gets complex, choose a clear hard-coded rule and name it as one.
6. **Present the rule before building it:**
   - the rule in one physical sentence;
   - the narrated moments that shaped it, including the ones it rules out;
   - the data values you think are false;
   - the open scope calls (e.g. ordered only or automatic too, which weapons, which sides), each with a recommendation.
7. **Build it with a test for each ruled-out moment,** not just the happy path. Keep the new state in replay and digest parity.
8. **Measure the shift; don't retune in the same pass.** Record the paired battle reports. Balance is a separate pass.

## Rules

- **Prefer properties over named cases.** A rule reads data columns (what stops rounds, what occludes, hit points, weight or push class, cover tier), never a kind's name. A new kind then inherits sane behaviour from its row.
- **Give each real-world effect one mechanism.** If two columns both model "hard to hit behind this", decide which one owns it.
- **A hold rule needs an exit.** Any "don't act because X" must name what clears X.
- **The user's "but what about…" belonged in step 3.** When it happens, add the moment you missed to your picture of the battle.

- For squad spotting thresholds, use living soldiers as the denominator in both fresh and retained sightings. Keep the unit's HIDDEN publication distinct from physical concealment: only a currently visible observer or known engagement clears the readout, while an unseen scout or stale acquisition-grace observer track does not. Reuse return-fire attacker memory for engagement rather than adding another timer. Forest ground owns the binary concealment bonus; tree crowns only attenuate sight lines.
