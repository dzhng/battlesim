# S7 — the voice queue reads the published steps (decision 16)

**Seam.** `packages/battle-audio/src/voices/responses.ts` `Arrivals`, fed by
`apps/battle-lab/src/unitResponses.ts`. Every accepted order kind is a queue entry;
journeys carry a group, non-journeys (attack, exit, return to base) carry none.
`observe` also takes each unit's published front step identity (kind plus target or
host id, never its point). A journey at the front ends on moving→stopped as today; a
non-journey ends when the front step changes, and never reports "arrived".

**Tests.** `[move A, attack X, move B]` with the attack chasing: "arrived" for A,
nothing during X, "arrived" for B only after the front leaves X; two consecutive attacks
end separately; the existing queued-journey cases stay green.

**Commit.** `feat(audio): the voice queue tracks every order step`.
