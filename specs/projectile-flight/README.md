# Projectile flight and visibility

Projectiles keep one engagement and visibility layer for their entire flight and
burst. It is the higher of the launch and target layers; deliberately lofted
ground-to-ground fire is LowAir. Ordinary bullet drop stays Ground. This rule
applies to every projectile, including bomb submunitions and interceptor rounds.
Authored interception and active-protection eligibility do not change.

The user clarified that low-arc grenades stay Ground. Only authored indirect
trajectories and top-attack climbs receive the ground-to-ground exception.

A missile gets one attempt. Losing tracking or missing beyond its tracking cone
ends homing irreversibly. It remains live, with normal physical impact and
lifetime behavior. Autonomous seekers do not depend on their launcher's sight.
Released rounds must not circle a retained aim point or reacquire a target.

Each side sees airborne paths and explosions through sensing in the assigned
layer. A visible projectile has a visible burst, including an air-to-ground
impact. This grants no ground vision, launcher identity, hidden-unit identity,
or casualty knowledge. Expiry does not gain a new explosion.

## Passes and dependencies

- [ ] Flight: reproduce stale-point circling; implement endpoint layers and
  one-attempt guidance, with regression tests written first.
- [x] Visibility (independent of flight internals): preserve layer evidence
  through paths and bursts; use it for side publication and missile warnings.
- [ ] Integrate: focused simulation checks, deterministic replay, actual visual
  evidence, and the complete repository closeout checks once.
- [ ] Whole-feature review, consolidated decisions ledger, archive rationale.

The first two passes run independently against the existing Round.layer field.
Keep flight and launch changes in one checkout; visibility owns observation and
flown-event publication. No shared build output between checkouts.

## Evidence and unresolved facts

The screenshot alone does not identify the weapon or prove its exact cause.
Existing guidance can keep steering toward an unavailable target's stale point.
Reproduce that mechanism deterministically and distinguish it from reproducing
the original live battle. Existing indirect and top-attack properties identify
lofted fire; surface any missing authoring policy instead of using names or
inventing height cutoffs. The launch acquisition cone currently checks launcher
facing, so in-flight tracking must use the missile's own flight direction.

Behavior changes need test-first regression proof, including unchanged defensive
eligibility, physical close hits after tracking loss, no reacquisition, side
visibility without ground leaks, and same-tick impacts. Visual verification needs
matched before/after captures and unprimed critique. Scratch evidence belongs in
throwaway, and the owning READMEs define commands.

## Next Agent Prompt

Implement the two independent passes above, then integrate and verify the whole
feature. Read development.md and the simulation README. Preserve every contract
above, keep meaningful new product choices explicit, and record implementation
decisions in decisions.md. The behavior-preserving battle-module split passed
all 19 authority tests and independent diff review. Visibility has red/green
proof for airborne flight and ground impacts, blind bands, same-tick round
removal and incoming warnings. All 201 sim unit tests and 41 shout tests pass;
independent review found no actionable defects. Integrate the delegated flight
pass next, then complete combined checks and visual evidence. The source exploration map
is in throwaway/projectile-flight-map.md in the initiating checkout.

Matched baseline captures are ready in throwaway/projectile-visual/before at
1920×1080, low graphics, ticks 90/300/600, with wide and strategic reference
framings. The frozen pre-change game WebAssembly is retained separately. The
existing air-survivability scene passes; its baseline loses the helicopter at
tick 845 and spends the first flare at tick 61. Rebuild WebAssembly only after
preserving these inputs, then run the same scene for the candidate.
