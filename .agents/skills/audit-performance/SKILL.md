---
name: audit-performance
description: Audit and prioritize performance work through bounded-work and forward-progress checks. Use when asked to find performance issues, investigate freezes/500s/OOMs, review retries or polling for no-progress loops, rank a performance backlog, or implement the next simple performance fixes.
---

# Audit Performance

Find work that grows without a useful bound or repeats without progress. Produce an evidence-backed
ledger or report, then prefer the smallest fix that preserves recovery.

## Workflow

1. **Trace hot paths.** Find recurring syncs, polls, streams, retries, queues, scheduled jobs,
   filesystem walks, and request-time reads. Follow each through its production caller; ignore
   test-only paths.
2. **Compute the amplification.** State the trigger and worst-case work in concrete units: rows,
   queries, pages, entries, bytes, jobs, retries, or full-file rewrites. Distinguish a bounded large
   constant from work that grows with history, tenants, paths, or outage duration.
3. **Check forward progress.** For every retry, fixed-prefix batch, cutoff, or transitional poll,
   identify what changes before the next attempt. If nothing must change, it can loop or starve
   forever. Check that partial fixes do not merely delay the same work.
4. **Falsify severity.** Inspect existing caps, indexes, backoff, deadlines, healing owners, and
   caller frequency. Dismiss findings already bounded cheaply enough or continuously healing.
5. **Record before fixing.** If the project keeps a performance ledger, append every supported
   finding and important dismissal using its existing convention. Otherwise return a compact report.
   Include priority, trigger, impact, current owner, acceptance seam, and evidence.
6. **Prioritize with the policy below.** If implementation is authorized, invoke the project's
   test-writing workflow and prove the old behavior red at the outermost practical seam. Fix, review,
   and update the ledger or report with verification evidence.

## Priority Policy

Rank highest when the issue can cause a user-visible freeze or error, memory/disk growth, blocked
request processing, data loss, fleet-wide amplification, or a poison item that prevents later work.

Prefer low-risk fixes that bound existing work: stream pagewise, honor backpressure, coalesce
schedules, move poison rows aside, skip proven no-ops, or make one exhaustive search end in a
terminal verdict. A safety cap is a pathology guard, not a normal product limit: set it above
legitimate large workloads, emit actionable telemetry when reached, and define what happens next.

Do not prioritize by scary-looking counts alone:

- Polling may continue forever when the polled state has a real recovery owner and can eventually
  change. Fix states that can never heal, not timers that are merely long-lived.
- Cheap bounded database work is acceptable without production evidence. Do not build a projection,
  cache, cursor state machine, or alternate read model just to reduce a modest fixed query count.
- Do not optimize information away when it may soon support product UI or behavior.

## Guardrails

- Preserve healing. A cache or no-op shortcut must retain cheap recovery signals and fall back to
  ordinary reconciliation on drift, uncertainty, restart, or prior failure.
- A recovery scan must either finish or persist forward progress. For a local, prunable namespace,
  prefer one complete pass with a ceiling high enough to indicate pathology rather than an ordinary
  large project. For a legitimately huge namespace, persist a cursor and resume after it. Finding the
  target updates identity; an exhaustive miss or abnormal ceiling produces the domain's terminal
  verdict. Never restart the same partial prefix forever.
- Separate retryable failures from terminal ones. A permanent rejection must not sit at a queue head
  or fixed prefix forever.
- Bound both sides of a transport and every durable/in-memory queue. State the overflow behavior;
  never silently drop accepted durable data.
- Measure sparse insertions and reordering in a long retained collection before accepting a delta
  protocol. Absolute addresses can turn one new row into a resend of most old rows; quiet-tick
  averages hide that amplification. Assert bounded edit bytes alongside exact reconstruction.
- Match compatibility work to the product lifecycle. In prelaunch code, prefer direct changes and
  add no legacy branches or migrations unless real persisted data requires them.
- Every limit introduced or changed must have an observable log or metric with the limit kind,
  configured bound, affected owner, and overload outcome.

## Done

The audit is complete only when every finding has a production trigger, quantified amplification,
priority rationale, acceptance seam, and recorded disposition; every dismissed candidate says which
bound or healing mechanism makes it acceptable. An implementation is complete only when its
red/green proof shows bounded work **and** continued recovery.

## Simulation report workloads

A report that accepts an arbitrary map must also locate its fight on that map.
An arena script with fixed depths from each edge can silently turn into two
disconnected rear-area moves when the world grows. Name complete-world loading,
local contact and edge-to-edge transit separately, and verify opposing goals
overlap before using the result as dense-combat evidence. Attribute fog traversal
and subsequent knowledge learning separately before selecting a sweep optimization.

When a sampled position clamps at an endpoint, test an all-rejected endpoint
inside the acceptance radius. A distance predicate then remains true forever.
Bound exhaustion by the finite source traversal, and fail a regression's injected
predicate after the legitimate attempts so the broken arm cannot hang the runner.

Process-wide instruction counters include every test thread. Isolate a committed
cost regression in a child process running only that test; a serial narrow command
during development does not prevent contamination when the full suite runs later.

Before accelerating a geometric candidate search, measure its failure arm and
check whether a necessary physical bound proves the whole result empty. Include
all allowed offsets (such as lean reach), retain numerical boundary cases, and
falsify both the negative work test and a positive boundary test.

For spatial cache invalidation, compare distant mutations with an unchanged
active view; a quiet steady-state sample misses global-revision amplification.
Check old and new moved footprints, deletion into an empty region and changed
heights against fresh complete queries. Count every instance of a shared index
when sizing metadata; tracking state unused by its other consumers is hidden
full-extent allocation.

For an integrated movement sweep, measure each unit's acknowledged destination,
including failed placement, rather than the group command's shared goal. A short
sample's unfinished long route is progress evidence, not a stuck verdict. Keep
proximity, pending/blocked/traffic states and death distinct; a dead unit's retained
movement enum must not accrue active or planning time in the report.
