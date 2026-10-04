# Native experiments and reports

Examples answer developer questions through the simulation's public owners.
They are not runtime services or a second set of game rules. Run an executable
with `cargo run -p sim --release --example <name> -- <arguments>`; its source usage
owns exact arguments and output schema. [This directory](./) is the current
inventory, while shared helpers under `common/` are not standalone commands.

## Choose the claim being measured

Tactical reports compare completed battles and digests. [Village comparison](village_report.rs)
provides quick iteration and full balance workloads; [cover calibration](cover_report.rs)
separates incoming scatter from physical interception. A changed workload or
configuration cannot stand in for an outcome-preserving comparison.

Physical diagnostic tools isolate motion or flight. [Movement shots](movement_shots.rs)
draw the real deterministic scenario table and can encode its animation with
ffmpeg. These top-down native pictures show paths and collisions; they do not
certify the WebGPU battle appearance. [Ricochet tracing](ricochet_trace.rs) reports
flown trajectories and actual hull outcomes rather than a damage-only probability.

Resource probes distinguish full-world construction, retained side knowledge,
publication and route search. [Navigation resources](navigation_resources.rs) and
[ground resources](ground_resources.rs) report stage-local allocation and work,
with explicit synthetic workloads and a shared [allocation accounting policy](common/allocations.rs).
Each executable owns its counters and process ceiling. Their scope
is not whole-process RSS or rendered GPU memory. Use a small declared workload
first; a refused or terminated probe is not a zero-cost result.

[Navigation quality](navigation_quality/) compares production routes against a
frozen dense planner. That duplicate is an independent reference, never a runtime
fallback. A route fit and cost comparison does not prove actual unit arrival;
movement and played-battle checks answer that separate question.

[Endurance](endurance_report.rs) and [city-scale reporting](city_report.rs) measure
long-battle and full-extent simulation workloads. Their accelerated native time
is not rendered FPS. Shared instruction counters separate load-independent work
from wall-time contention; [the browser benchmark](../../../web/src/battle/benchmark/README.md)
owns real frame cost. Keep scratch output under `throwaway/` and preserve the
printed workload identity alongside a comparison.

## Tools called by editors and map preparation

[Mechanics validation](mechanics_validate.rs) accepts supplied JSON on stdin and
returns canonical resolved data without editing fixtures. It is built and invoked
by [the mechanics editor](../../../apps/mechanics-editor/README.md), not called as
a textual report. [Encounter reporting](encounter_report.rs) plans on an admitted
saved map; its explicit save mode writes an encounter, while inspection output
alone does not publish one.

[Mapgen examples](../../mapgen/examples/) own generation sweeps and the map editor's
native report. The [map workbench](../../../apps/map-workbench/README.md) captures
inputs and retains artifacts; manually changing those artifacts cannot alter a
reviewed server-held save candidate.
