# 03 — Scenario corpus and tournament

This slice makes policy quality measurable and gives future policies a plug-in
surface without shipping multiple production policies.

## Corpus

Add a fixture-owned corpus with valid 3/5/7-site skirmish setups. Include:

- flat opening and zero-unit preparation;
- town contact and wooded contact;
- blocked entry/route failure;
- contested centre and multiple objectives;
- casualty/suppression and logistics pressure;
- helicopter reinforcement and airborne contact;
- generated open, mixed, and metro seeds with mirrored factions.

Rows name map/encounter identity, factions, battle seed, policy id, and bounded
horizon. Qualitative expectations cover recon envelope, legal counter response,
eventual pivot eligibility, and progress; exact winners remain report evidence.

## Harness

Build a native deterministic tournament runner, preferably beside the existing
map-generation battle sweep because it already owns generated-map admission and
JSONL/refusal reporting. The first runner sets `ai_side` to none and drives both
sides externally through public observation, side-scoped queries, command
acceptance, and stepping. Add a two-side setup controller only if this host
cannot remain equivalent to live play.

The runner reports complete/refused/timeout/no-progress status, result, admitted
map identity and site count, objective ownership time, contact timing, recon
sightings/range, doctrine pivots, refused orders, credit spend, living cost,
battle/replay digests, and replay parity. Instruction counts remain a future
measurement extension. Trace is non-authoritative and stays out of
publication/digest.

## Verification and closeout

- Repeat one row and get identical commands, report, and replay digest.
- Run mirrored side assignments on the same admitted map identity.
- Keep refusal and timeout rows explicit; never treat them as wins or zero-cost
  matches.
- Run the quick corpus during iteration and the full corpus once at closeout.
- Run workspace `check` and browser verification at feature closeout.
- Run `review`, then `close-spec` to archive this plan as durable rationale.

