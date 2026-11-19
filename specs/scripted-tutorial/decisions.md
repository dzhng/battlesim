# Decisions ledger

What was decided, by whom and why. Append entries; never rewrite one. Each slice
records the decisions it had to make that the plan didn't, with the reason.

## Interview (user, 2026-11-19)

- **Guide.** The old page is the Guide, reached only from Esc as a centred
  modal. The main menu's Tutorial tile starts the battle.
- **Story.** Operation Ferryman ([story.md](story.md)) was approved as drafted.
- **Scope.** All seven chapters (recommended option), including supply,
  artillery, helicopters, jets and SEAD.
- **Structure.** One continuous battle. Restart chapter replays the recorded
  battle to the chapter's start (recommended option).
- **Map.** A fixed saved map. Asked about a `tutorial` category, the user said:
  "name it to be something more generalized, because I do want to create
  explicit scenarios with explicit maps in the future". So the category is
  `scenario`, and the format is general.
- **Test gate.** A headless walkthrough to Victory, with a lopsided design
  (recommended option).
- **Narration and avatars.** The user asked for Bento to create narrations and
  any assets needed. Voice: Eryn, picked from five library voices (Eryn, Pam, Brody, Rolando, Philip), all reading the same opening line. Portrait:
  "match the voice's gender", so the agent picked F1, which faces the camera.
  Animation: a still portrait with comms effects (recommended option).
- **Compatibility.** Neither backward compatibility nor migrations (the
  write-spec default; the user didn't opt in).

## Narration assets (agent, 2026-11-19)

- **Library voices only.** Bento's ElevenLabs proxy doesn't admit
  `/v1/text-to-voice/design`, and voices created through it expire after 24 h.
  Shared-library voice ids work directly in text-to-speech and don't expire, so
  any line can be re-rendered later in the same voice.
- **58 lines** were rendered in the `armata` workspace with `eleven_v3`, seed 7.
  Every take passed a speech-to-text check word for word; the only differences
  were spellings like armour/armor and treeline/tree line. They are review takes
  in [assets/narration/takes](assets/narration/takes); slice 10 brings them into
  the voice pipeline. The scratch tools that rendered them (render.sh,
  check.sh) are in `throwaway/tutorial-assets/narration/`, and slice 10
  replaces them with the pipeline's own generator.
- **Lines carry intent, never keys or numbers** (see [story.md](story.md)). A
  spoken key name would go stale with a rebinding; a spoken number with a
  retune.
- **No shoulder-launched air defence in the `us` roster file** (its air
  defence is the M-SHORAD, Patriot and NASAMS; draft C reports a Stinger team
  in the shared roster). The chapter 6 line stays generic ("get air defence
  up"); P4 picks the card.

## Synthesis

Three blind drafts from the same brief: A (fewest slices, Claude), B (risk
first, Codex gpt-6.2-sol), C (seam quality, Claude). The drafts are in
`throwaway/tutorial-drafts/` until this spec closes.

**All three agreed** on: probes first (map, mixed skirmish and scripts, replay
cost); one runtime contract (`ScenarioDefinition`); a shipped scenario follows
current rules (never a frozen bundle); grants computed from current prices; an
offer list enforced by the sim and mirrored by the shop; beats published only to
their side; the coach as client presentation reading only its own side; restart
as a replay prefix plus going live; a native walkthrough; the deploy key is T.

**Where they split:**

| Question | A | B | C | Call | Why |
|---|---|---|---|---|---|
| Who runs the story and red's cues | Authored scripts, extended with event memory, tags and places | Authored scripts plus declared public beats | A typed director (triggers and actions) in the sim, digested, rerun on replay | **C** | The interpreter is stateless, position-addressed, can't buy, unload or garrison, and isn't published. A typed director is checked at admission, the coach validator can see its beat list, and orders reuse the fixed-script path. The interpreter stays the bundle's custom-code surface (named follow-up in README). |
| Coach ↔ sim link | Recorded `Advance { stage }` from the coach; stages gate red | Coach reducer waits on public beats and UI; no command | The director never waits on the coach; the coach pauses the offline battle for cards | **C** | Progression never depends on presentation. A UI step that must not let time pass pauses the offline battle, which changes no determinism. Chapter openings come from the director's digested state, so restart needs no extra marker. |
| Scenario source format | Extend `EncounterDefinition` | Extend `EncounterDefinition` | New `ScenarioDocument`, no `overrides` by type | **C** | A minority call, made deliberately. Encounters are film and test staging that may pin stats; a player scenario must not. Keeping each type's invariants in its fields beats a category-dependent refusal. The loaders share the map-loading code, and only the small document loader is new. |
| Red seat behaviour | A red director script on a red player | A red authored script | `Playbook::Directed`: fights by ordinary target choice at authored engagement, moves only on director cues | **C** | `PassiveAi` forces return fire, so neither the ambush tank nor a suppressible outpost works under it. Playbooks also consolidate four ad-hoc opponent drivers. |
| Walkthrough | Rust solution code against the scenario | Browser/test driver sending normal commands | Solution beats in the director's vocabulary, the human seat `Directed` at test time | **C** | No test-only runtime path; the walkthrough also exercises the director's order path; the solution is data beside the scenario. |
| Bundle | Untouched | Add `ScriptedEconomy` mode | Untouched; export is resolve then wrap | **A/C** | Nothing in this feature needs a new bundle mode. |
| Slice count | 4 | 9 + 4 probes | 25 | **19** | C's seams, merged where one gate covers two seams (05, 06, 11), and A's restart-before-chapter-3 ordering. |

**Picked up from a single draft:**

- A: P5, the suspected seq bug: player-owned authored script orders are stamped
  with the side's sequence number while admission checks the player's own.
- A: the restart cost must be reported as a loose tripwire, not a target.
- C: `MapCatalog::scenario` → `encounter_scenario`.
- C: per-chapter recovery beats against soft locks.
- C: camera tours reuse the capture tool's reel format.
- B: compare the browser publication overhead of a fast-forward, not only native
  tick cost (P2).

## Map generation hills (user, 2026-11-19)

The user pointed out that map-generation hills are being worked on in a separate
session ("Map Gen Hills Status and Spec", branch `t3/map-gen-rolling-hills`),
and said to coordinate with it if needed. On `staging`, generated maps have no
hills. The ground-sight check refuses relief, and the branch caps hills at 8 m
until a crest counts as cover. P1 now checks for that work first and falls back
to authored relief on the saved map, so the tutorial never blocks on it. The
hills session was told what the valley needs.
