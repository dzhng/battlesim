# Decisions and scope rationale

## Fixed decisions

| Decision                                                                                                             | Rationale                                                                                                                                                                                                                              |
| -------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Preserve behavior, public formats and deterministic outcomes; hard internal cutovers, no compatibility or migrations | Explicit user answer on 2026-11-06. “Comprehensive” means account for the complete audit and implement every justified cleanup.                                                                                                        |
| Select ownership defects, not every implicit machine                                                                 | A timestamp, optional resource or derived label can already be the simplest representation. The scope ledger closes every audit row.                                                                                                   |
| Use local domain records and reducers; add no FSM dependency                                                         | Existing TypeScript/Rust types and React already provide the required tools. A shared engine would add an abstraction without a common transition policy.                                                                              |
| Keep request families separate                                                                                       | Command, preview and advance IDs have separate namespaces; replay has no request ID. A universal ID-keyed pending map would need tags/casts or wire redesign. Only the duplicate advance target map is merged into its request record. |
| Keep purchase and move-preview schedulers separate                                                                   | Their freshness and re-query timing differ. Reuse type vocabulary only when it describes the same fact; no common latest-request framework.                                                                                            |
| Keep existing GPU, journal, simulation and audio-policy owners                                                       | Their queues, timers, phases and atomic commits already describe actual independent dimensions. Changing their representation would not remove scattered ownership.                                                                    |
| Split overloaded consumers before extracting view components                                                         | Each extracted module must own a coherent transition/resource policy. Moving JSX or callbacks solely to reduce line count would leave the complexity intact.                                                                           |
| Retain page GPU lifecycle as currently owned                                                                         | AppResources and BrowserGpu already have clear page-versus-scene ownership, admission identity and disposal gates. A tagged rewrite alone is not selected; its tests protect viewport changes.                                         |
| Refactor the authority resource pair, not its scheduler                                                              | Module/battle admission and teardown can have one record instead of correlated nullable resources. Pause, visibility, scripted stepping and credits remain independent and pump precedence remains local.                              |
| Refactor map artifact retention, not the service queue or file transaction                                           | Artifacts have lifetime rules independent of the current reviewed candidate and bounded request queue. The journal remains the sole publication authority.                                                                             |
| No simulation-phase cleanup in this plan                                                                             | The audit found strong existing component owners and important overlaps. Gun top-up, deployment progress, movement replanning, AI commitments and terminal evidence must not become a giant unit/battle enum.                          |

## Synthesis and recursive slicing

Three independent drafts examined the complete audit, real owners and tests: a
fewest-slices Codex draft, a seam-quality Codex draft and a risk-first Claude draft.
The latter is available at
<t3-thread://v2/9950c6b4-6db9-4544-9904-e8f629486d1f/thread:delegated-task:command%3Amcp%3Acdbb94b3-c200-4d34-a874-96e9d1cc0c87%3Adelegate-task%3A60bf7384-ec86-418f-ab19-4a9987553274>.
The canonical decisions are here; draft proposals are not additional instructions.

All drafts favored preparation deduplication, client request clarity, independent
button lanes, editor operation ownership and page/battle audio separation. They
agreed that gun reload and deployment are already well modeled. The final plan
keeps those existing owners rather than interpreting agreement as a reason to
rename every state.

The minimal draft grouped several seams into large passes; the seam draft exposed
where those passes hid separate questions. The final ladder separates gesture
recognition, purchase admission, move/building preview and release acknowledgement;
scene admission from submitted picking; map draft/display from manual jobs and
server retention; page audio from battle graph; and sound review from audition.
Each slice now has one principal question and a focused consumer verdict.

The risk draft proposed one universal pending-request table and several small
enum conversions. These were not adopted: request namespaces differ, nested menu
panels are simple local state, and graphics detection already has a tested owner.
The risk findings about settlement, graph identity and multi-button behavior are
preserved in the contracts and owning slice tests. No conditional “convert this
later if useful” backlog remains in the implementation ladder.

## Behavior questions outside this refactor

These are source-derived questions, **not reproduced failures or approved behavior
changes**. Each has a firm default: preserve the existing behavior in this plan.
They do not block implementation of the selected owners.

| Question                                            | Evidence / boundary                                                                                                                                                                                     | Decision here                                                                                                                                                                                                                                                                                                                              |
| --------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Ordinary authored victory versus replay terminality | `run_authored_scripts` can record victory evidence without a terminal tick; `apply_script_trace` gives a true victory trace a terminal tick in [battle.rs](../../crates/sim/src/battle.rs).             | Retain audit 66. A future correction must establish intended script semantics and exact live/replay evidence; never collapse outcome and terminality during cleanup.                                                                                                                                                                       |
| Concurrent replay requests                          | The simulation client has one replay waiter and untagged replies.                                                                                                                                       | Preserve current single-waiter behavior. No unrequested correlation-ID extension, coalescing or FIFO semantic change.                                                                                                                                                                                                                      |
| Uniform disposal settlement                         | Client previews reject on dispose while other pending operations remain silent; preparation is also intentionally silent.                                                                               | Keep the current family-specific policies. Any uniform rejection change must update every waiting continuation and prove no unhandled rejection.                                                                                                                                                                                           |
| Pause ownership after client replacement            | The menu retains a resume-ownership ref independently of client identity.                                                                                                                               | Retain audit 21. If a real wrong-client resume is demonstrated in an affected journey, separate it as a correction rather than changing menu state representation opportunistically.                                                                                                                                                       |
| Build errors across option changes                  | The async scenario hook keys success but not its error payload.                                                                                                                                         | Retain audit 11; stale-error behavior needs a focused consumer regression before any correction.                                                                                                                                                                                                                                           |
| Counted stream error/cancellation                   | Download accounting finishes on normal transform flush.                                                                                                                                                 | Retain audit 14; abnormal stream completion is a separate behavior investigation, not a reason to introduce a general download FSM.                                                                                                                                                                                                        |
| Late import or audition completion                  | Several UI flows await external reads and resource preparation before publishing fields.                                                                                                                | Selected slices preserve current ordering and identity gates. A newly demonstrated stale mutation is fixed only through the correction procedure below.                                                                                                                                                                                    |
| Submitted-view replacement admission                | A replacement scene can be installed before its first successful draw; the previous implementation kept the retired scene's picks live and could leave staged work attached to the retired scene.       | Named correction in slice 11: replacement scenes withhold picks and drop staged work until their first successful submit. Affected consumers are viewport picking and frame completion; the invariant is that both refer only to the scene actually drawn. `viewportReplacement.test.tsx` pins scene identity and skipped-submit behavior. |
| Submitted-view replacement admission                | A replacement scene can be installed before its first successful draw; retaining the retired scene's picks or staged frame would let input and frame completion refer to resources no longer on screen. | Named correction in slice 11: picks are withheld and staged work is dropped until the replacement submits. `viewportReplacement.test.tsx` pins the scene identity and skipped-submit behavior.                                                                                                                                             |

If implementation demonstrates a defect, record a named correction with trigger,
current outcome, intended outcome, affected consumers and invariant impact; write
one failing behavior test first. Isolate its commit from the structural change.
Routine fixes with an established repository contract are authorized work. A
choice between genuinely different game rules or externally observable semantics
must be resolved explicitly and the spec updated before dependent implementation.
Continue independent authorized work while such a decision is pending.

## Deliberate implementation freedom

Private names, placement of short helpers inside the named owner, and small
cohesive file splits are delegated. Slice files prescribe records versus reducers
where that is part of the architecture. Test authors may choose the smallest
existing fixture or controlled edge that proves the named behavior.

Cancellation, timing, retries, request namespaces, resource retention, concurrent
operation policy, public formats, gameplay, animation and rendering order are not
delegated. An unlisted architectural choice is a spec gap: update the owning slice
and this ledger before broadening the implementation.
