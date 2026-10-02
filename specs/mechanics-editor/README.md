# Mechanics editor

The local editor lets the developer tune the game in battlefield terms while
the existing authored fixtures remain the source of truth. The default is a
searchable spreadsheet of concrete units; expanding a row exposes explained
gameplay fields and the weapons that unit carries.

## Next Agent Prompt

Status (2026-10-01): native admission, spreadsheet controls and generation capture
are integrated. The server publishes validated JSON and restores inherited
soldier and mount overrides. Finish the integrated browser proof on a scratch
checkout, then run the closeout gates and archive this spec. Preserve the shipped
grenade speed of 100 m/s, range of 150 m and scatter of 30 mrad.

Current pickup: rerun the scratch browser proof against the corrected canonical
publisher, finish search/preview/error captures, then run the whole-feature review,
closeout gates and final choices ledger.
Browser Save, reciprocal spread inputs, active battle isolation, explicit restart,
invalid flight, outside-edit protection and inheritance restoration passed on
scratch fixtures. Metal Chromium was required for unchanged GPU admission; effect
PNG assets needed local LFS checkout. Server tests now cover exact Rust catalog
bytes and edits made during publication. Full closeout gates are outstanding.

- [ ] [Admission and publication](slices/01-admission.md).
- [ ] [Spreadsheet and explained editing](slices/02-editor.md).
- [ ] [Battle lifecycle and integrated proof](slices/03-lifecycle.md).

Slices 1 and 2 run in parallel against the shared editor protocol. Slice 3
integrates them. Use focused tests during implementation. The full repo gates
run once at closeout, with browser coverage restricted to the new editor and
the battle lifecycle it changes; unrelated visual scenes cannot be changed by
this feature. No battle balance run is needed: shipped tuning stays unchanged.

## Contracts

Save replaces authored JSON only after preview and validation, preserves
inheritance, rejects outside edits and synchronizes the generated catalog.
The backend owns file paths and formats JSON. The Rust catalog and Rules
resolvers own admission; the flight validator checks coupled weapon settings.
Preview never writes. Publication is serialized, stages replacements, rolls
back failures and recovers interrupted publication before serving readers.
Individual file replacements are atomic; multi-file filesystem changes are not
an OS transaction. The served generation becomes available only on completion.

Unit edits create local overrides with restore controls. Shared weapon edits
affect every user, shown before save. Soldier-derived edits create a soldier
variant and bind only the selected unit's slots; no new catalog schema is
needed. A field masked by an upgrade part must be reported rather than silently
discarded. Models and art are outside editing scope; physical dimensions remain
gameplay fields, with model-fit implications disclosed and existing checks used.

Controls use human units. Landing spread is one-axis standard deviation in
metres at maximum engagement range, not a maximum miss or blast radius.
Angular scatter is calculated for persistence. Range edits preserve landing
spread. Fractions use percentages, firing intervals use rounds/minute, and
half-extents use full dimensions. Each conversion explains and exposes its
stored value. Both raw and computed values are editable side by side and update
each other immediately; invalid/partial text remains visible and blocks saving.
Derived values must round-trip without tuning drift.

Saving does not restart active battles. New or explicitly restarted battles
capture the latest accepted rules and matching presentation catalog. Vite must
not reload running battle pages because the editor saved fixture files.

## Ownership and review

The editor is a source-only app under `apps/`, served through the existing Vite
dev server. Its HTTP API is development-only and only accepts local, same-origin
requests. No new dependency, separate server, compatibility layer or migration.
The existing development command remains the entry point; a developer menu link
opens the editor. Naming, internal structure and reversible styling are delegated.

Every visual pass requires actual browser captures, compare-screenshots against
the stated spreadsheet/expanded-form requirements, an unprimed screenshot-critique
as the last visual check, and preview-shots for the user. Keep scratch evidence
in `throwaway/`. Run review (shape, diff, docs), audit choices and commit each
clean checkpoint; continue until all contracts are proved, then close-spec.

## Exploration map

User decisions: replace source JSON; globally edit shared weapons; expose all
existing gameplay values; new/restarted battles apply saves; spreadsheet first
with search and expansion; inherited unit edits stay local; edit landing spread
directly and convert awkward stored units automatically.

Code findings: infantry carry shared soldier definitions; named mounts merge by
name; parts apply after inheritance; the browser catalog is generated; Rules
deserialization alone misses flight-profile validation; static fixture imports
can trigger HMR and cache old data during restart. These findings shape the
contracts above. The remaining work is implementation, not a product question.
