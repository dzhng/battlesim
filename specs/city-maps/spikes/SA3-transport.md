# SA3 transport: exact runs and atomic publication admission

**Verdict:** the integrated run serializer, decoder and receiver pass focused preservation
checks. This unlocks their production seam. Full-size native all-touched/churn overlap,
arbitrary-entropy admission, active-battle throughput and the remaining GPU matrix stay
open in SA3/S1/C07. Complete G0 and deferred artwork acceptance remain open.

## Ownership and format

`KnownGround.change_runs_since` is the single borrowed run source; Publisher writes it
without collecting native cell patches or a second run array. The layout publishes its
tile size, fields and atomic record limit. A record contains the unchanged observation
groups, fog payload and four-float ground runs. Each run retains exact raw ground bytes,
its learning cursor, a tile ID and local cell span. Partial edge holes are never values.

The production serializer is also the native adversarial codec test surface. The wasm
JSON-only packing hook and old cell-patch types/iterators are removed because their only
consumers were tests. [Frozen codec evidence](../assets/ground-transport/README.md) keeps
the original animation coverage without another production protocol. The existing frozen
ground and complete observation oracles are preserved, not repinned.

## Admission and failure

The publisher computes the complete encoded size with checked arithmetic before reserving
or clearing output. The provisional atomic allowance is 64 MiB: finest-grid Large fog
and one uniform run per full ground tile each cost about 20.25 MB, leaving room for the
other observation groups. This calculation is a codec bound, not proof that every
high-entropy world or active city fits it. Beyond the allowance, publication returns an
explicit error; cells and observations are never truncated or reported as empty.

Pre-admission rejection preserves the previous complete output and all producer cursors.
Output reservation failure is returned as an error. Auxiliary fog-difference and retained
baseline allocations are not covered by that reservation guarantee. A late internal
count/encoding invariant failure clears the incomplete output; the production iterator is
immutable across counting/writing. Do not describe all failures as retaining the old record.

The decoder validates both ground and fog baselines before committing either one. A
ground revision may remain unchanged while fog advances. A receiver exception or authority
error fails the client once, pauses production, returns held/incoming credits once, and
rejects pending command, advance and replay promises. Later status replies cannot revive
that failed client. A new client is the recovery boundary; no automatic resynchronization
loop is introduced.

## Executed checks

- A real small Battle with uniformly saturated learned ground originally produced
  16,840 B. The compact publisher passes the under-2,048 B contract and identical resync.
- Eight publication, eight ground-delivery and four native ground-page tests pass,
  including exact wide IDs, final fog padding, frozen digests and edge holes.
- The integrated wasm passes ground/foliage preservation and complete retained movement
  observations. Nine focused web suites pass, covering the authority, decoder, learned
  pages, sampling and animation consumers; the additional held-credit regression passes.
- The broken paired-cursor regression failed before decoder correction. Restoring early
  output clearing fails the previous-record assertion; restoring 32-bit fog multiplication
  incorrectly accepts the oversized field and fails the bound regression. Omitting held
  credit return fails the runtime error regression. Corrected paths pass.
- Typecheck and focused clippy pass. The configured Codex CLI model is unsupported by its
  ChatGPT endpoint, so its attempted review supplies no finding or acceptance. Independent
  agent review identified the paired-cursor and stranded-credit bugs; both are corrected.

Whole check/verify runs belong to the combined representation closeout before merge.
This report supplies no new visual design or art approval.
