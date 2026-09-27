---
name: tweak-mechanics
description: Change or add a game mechanic or rule safely — who can shoot, see, move, block, push, destroy, take cover, or be hit. Use before proposing or implementing any rule change, a new body/prop/unit property, a fix for "units behave wrong", or a change that came out of a balance finding.
---

# Tweak Mechanics

A rule change is a **blast radius** problem. It applies to every actor and every subject it touches, and it composes with every other rule on the same path. Find the unwanted cases yourself, before the user has to ask about them.

## Workflow

1. **Confirm the cause in the code.** Reports paraphrase, and they can be wrong about the mechanism. Read the rule that actually decided the behaviour, and state it in one sentence: "X happens because rule R returns Y when Z". **Tell:** a fix proposed for a mechanism nobody has read.
2. **Build the grid: actors × subjects.** Rows are every kind that *acts* under the rule: every weapon, unit or order type. Columns are every kind it *acts on*: every prop, body or unit row with the property the rule reads. Each subject plays two roles, so give each an outcome per actor:
   - **as the target:** the thing the actor meant to act on;
   - **in the way:** incidental, standing between the actor and some other target. A rule meant for "the thing in the way" lets the actor reach whatever that thing hides: an enemy it can't see, a friendly unit, the objective. Say what it reaches, and whether that's wanted.
   
   Then audit the subject's **data**, not the rule. For each property value the rule reads, ask the real-world question. Would this stop a bullet? Could you see through it? Would a truck push it? Flag every value that's false; a wrong value multiplies through every rule that reads it.

   A tiny example, from a different mechanic ("vehicles push light props aside"):

   | subject | as the target | in the way | data true? |
   |---|---|---|---|
   | crate | pushed, fine | pushed into a soldier standing behind it: wanted? | "light", true |
   | parked truck | n/a | pushed aside by a tank: is a live vehicle a prop here? | "medium", true |

   **Done when** every subject has both roles filled for every actor and its data audited, not just the pair that prompted the change.
3. **Attack the grid.** For the high-impact cells (the big bodies, the objective, garrisons, anything that hides units), write at least one concrete battle moment per cell, from both sides:
   - what the player sees;
   - what the enemy can exploit;
   - what a player would call absurd.
   
   Name the **intended subject** of the action, and ask what happens to **incidental** ones: a rule written for "the thing in the way" also fires on things you'd never want to act on. "None found" is not an answer. If a cell truly has no problem, say why in one line.
4. **Walk the neighbouring rules.** For each other rule on the same path (sight and occlusion, cover and concealment, blocking, destruction, targeting, orders against automatic behaviour, knowledge and fog), check three things:
   - **double counting:** two mechanisms modelling one real effect;
   - **freezes:** a rule that makes a unit wait on something that never clears;
   - **degenerate play:** a cheap trick the rule rewards.
5. **Keep it first-principled, and pragmatic.** Rules read bodies, footprints, weight, and what a round or vehicle can break. If the principled version gets complex, choose a clear hard-coded rule and name it as one.
6. **Present the rule before building it:**
   - the rule in one physical sentence;
   - the grid;
   - the cases from steps 3–4, each with the behaviour you recommend;
   - the open scope calls (e.g. ordered only or automatic too, which sides, which weapons), each with a recommendation.
7. **Build it with a test for each unwanted case,** not just the happy path. Keep the new state in replay and digest parity.
8. **Measure the shift; don't retune in the same pass.** Record the paired battle reports. Balance is a separate pass.

## Rules

- **Prefer properties over named cases.** A rule reads data columns (what stops rounds, what occludes, hit points, weight or push class, cover tier), never a kind's name. A new kind then inherits sane behaviour from its row.
- **Give each real-world effect one mechanism.** If two columns both model "hard to hit behind this", decide which one owns it.
- **A hold rule needs an exit.** Any "don't act because X" must name what clears X.
- **Surface the unwanted cases in the proposal.** When the user's first reply is "but what about…", that question belonged in step 3.
