# Preservation contracts

These are the invariants every slice inherits. Source links name the current
contract owners; proposed internal records are specified in the slice files.

## Boundaries

| Boundary                                                                | Preserved contract                                                                                                                          |
| ----------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| [Worker protocol](../../../web/src/battle/sim/protocol.ts)              | Same request/reply shapes, issuer sequences, verdicts, transfer ownership and ordering. No version negotiation or new IDs on the wire.      |
| [Preparation](../../../web/src/battle/prepare/client.ts)                | One worker prepares and then runs the battle. Cancelled pending preparation stays unsettled and silent, including the uploaded-bundle path. |
| [Publication reader](../../../crates/sim/src/publication/reader.rs)     | Same epoch, observer, baseline transaction and raw packed bits; no hidden knowledge added.                                                  |
| [Replay](../../../crates/sim/src/battle/replay_identity.rs)             | Same saved shape, identity validation and accepted-command semantics. Do not bypass cross-build rejection.                                  |
| [Renderer runtime](../../../crates/renderer-core/src/runtime/mod.rs)    | Same canonical frame, incremental work and requirement for a fresh frame after catalog commit.                                              |
| [Audio](../../../packages/battle-audio/src/appAudio.ts)                 | Same permission retries, mix, start/stop timing, seeded score, observations dropped before readiness, and stats fields.                     |
| [Fixture publication](../../../apps/fixture-publication/publication.ts) | Same reviews/receipts, HTTP payloads, journal bytes, serialization, rollback and outside-edit refusal.                                      |

No schema migration is planned. Internal callers cut over in the owning slice;
no alias, adapter or second state representation remains to protect obsolete code.

## Orthogonal dimensions

- Authority lifetime, started intent, pause intent, document visibility, script
  target, available credits and terminal battle outcome are independent. A status
  string is a projection, not the scheduler's new authority.
- The authority's pump keeps its existing order: reject non-live/not-started work;
  terminal check; scripted advance; terminal recheck; hidden; paused; credit
  recovery; bounded real-time catch-up. Scripted work can run while paused.
- A purchase choice, pending preview, retained answer and already-sent confirmation
  can coexist. Cancelling the choice does not undo the confirmed command.
- Purchase answers remain usable across newer knowledge revisions at the same
  spot while another answer is pending. Movement/building preview freshness has
  its own policy. Purchase re-queries in completion; pointer preview re-queries on
  the next frame. Do not consolidate these into one scheduler.
- Left press, right press and middle-button orbit occupy independent lanes. Keep
  existing overlapping-button behavior, HUD ownership, modifier capture, press-ray
  versus release-ray meaning, and hover after normal capture release.
- Requested input, active operation, last accepted result and displayed result
  are different. Failed new map/art work must not accidentally clear retained
  admitted output. Scene topology replacement and asset-only admission are separate.
- Browser audio permission, mute, context failure, menu preparation failure,
  menu playback and battle graph readiness are separate. A failed graph/device may
  still require disposal. Resource identity must survive until cleanup ends.

## Request settlement and timing

The [simulation client](../../../web/src/battle/sim/client.ts) currently distinguishes
failure from disposal. Preserve this distinction rather than giving every promise
the same termination policy:

| Event                    | Existing outcome to preserve                                                                                                                                        |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Authority/client failure | Return held publication buffers once, send pause, reject ready and every currently tracked request, publish failed status.                                          |
| Client disposal          | Reject pending previews, close transport, detach visibility listener, publish disposed status. Pending ready/command/advance/replay promises are not newly settled. |
| Preparation cancellation | Terminate owned worker and ignore late messages without settling pending preparation.                                                                               |
| Invalid building preview | Reject that query; leave authority and unrelated commands usable.                                                                                                   |
| Advance acknowledgement  | Record target; resolve only once released publication tick reaches it, in either message/release ordering.                                                          |
| Publication release      | Return a credited buffer at most once. Holding all credits stalls simulation; return resumes from current time rather than repaying stalled wall-clock debt.        |

Tests must observe bounded absence of a completion using controlled microtask/
message turns; never leave a test awaiting an intentionally silent promise.
Current `replay()` is a single-waiter API. The cleanup keeps that policy; adding
concurrent replay semantics is a separate correction, not a generic RPC-map side
effect. Commands, preview IDs and advance IDs have distinct namespaces.

All existing queue bounds, retry eligibility, deadlines, debounce, frame cadence,
audio scheduling and error categories remain intact. Representing a timeout or
status more clearly does not authorize retuning it.

## Single ownership after cutover

| Concept                                  | Owner after this plan                                                                          |
| ---------------------------------------- | ---------------------------------------------------------------------------------------------- |
| Pending command/query/advance settlement | Existing simulation client, with typed family records and one terminal-policy implementation   |
| Prepared worker handoff                  | Preparation-local worker owner reused by both preparation entry points                         |
| Admitted simulation resources            | Existing authority lifetime record; pump still owns execution                                  |
| Session startup and warm-up continuation | One session-local operation; React projects its current session                                |
| Pointer press recognition                | Viewport-local input owner; DOM adapter owns event/capture plumbing                            |
| Purchase admission                       | Existing purchase controller with separate captured preview/confirmation operations            |
| Move/building preview                    | Local order-preview controller; paint consumes its result                                      |
| Released order acknowledgement           | Session-local released-action record, separate from the live gesture                           |
| Scene admission/replacement              | Viewport-local scene owner; existing SceneAppearances still owns asset-only replacement        |
| Submitted view/picks                     | One snapshot installed only after successful draw; ViewportFrames still owns waiter completion |
| Page/battle audio resources              | Existing AppAudio/BattleAudio, each carrying its own complete graph and failure obligations    |
| Mechanics/map/sound drafts               | Separate domain reducers with operation controllers; no universal editor reducer               |
| Map artifact lifetime                    | Map-service-local artifact owner, called by the existing bounded service queue                 |
| Audition resources                       | Existing SoundAuditioner owns each run; UI status projects that run                            |

Do not introduce a new shared package, event bus, global store, statechart runtime,
generic resource registry, lifecycle superclass or permanent tracing facility.
Reuse an existing owner where it fits. A helper shared by the two preparation
paths is justified by the identical worker handoff, not by superficial similarity
to GPU or audio ownership.

## Behavioral corrections

The user explicitly chose preserved player behavior, public formats and battle
outcomes, with hard internal cutovers and no compatibility scaffolding. A refactor
must not silently “fix” a surprising policy. Follow the separate correction
procedure in [decisions](decisions.md#behavior-questions-outside-this-refactor).
In particular, no simulation terminality or gameplay decision is delegated here.
