# 13 — Closeout

**Unlocks:** the feature is finished and recorded.

## Steps

1. Run everything once: the full `check`, browser `verify`, and the balance report.
2. Scratch roster smoke: run the catalog smoke routine (`every_unit_type_sets_up_fires_each_mount_and_moves`)
   over the twelve admitted cards on the game set, uncommitted — no committed test walks the
   roster, by the catalog tests' design. Every mount fires, every card reaches its goal.
3. Tune general parameters only (cost, the Javelin/Akeron penetration and loft, light-tank
   armour) against the report. One pass, then a rerun of just what the change could move.
4. Measure the game page's catalog download against `asset check`'s budget with all twelve
   models in. Record it.
5. One skirmish capture with the new units beside their peers, critiqued unprimed with
   [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md); open it for the
   user with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) (non-blocking).
6. Update [the admission audit](../../done/unit-roster/capability-admission.md): its
   "Trophy blocked" and "top attack deferred" rows point to this feature's rationale.
7. Confirm the end state in the README's Single owners section: no `run_disabled` for an
   admitted card, no `disabled/` source for one, no exporter-local frame for a live card.
8. Run [review](../../../.agents/skills/review/SKILL.md), then
   [close-spec](../../../.agents/skills/close-spec/SKILL.md). Push.
