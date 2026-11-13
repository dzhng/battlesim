# 10: sound

## Contract

When a side can see a cloud within its bloom time of the pop, it hears that
cloud's pop at the cloud:
- **tanks and IFVs:** a discharger thump-thump, then a hiss;
- **infantry:** a grenade pop and a hiss.

A side that first sees the cloud later never hears the pop. A pop creates no
contact, and the hearing cues are unchanged (D16).

## Seam

- **Clips.** Generate the clips with ElevenLabs through Bento, the same
  generator the flare burst used (`fixtures/sounds.json`,
  `eleven_text_to_sound_v2`).
  - Extend the prompts in `packages/battle-audio/tools/generate/effects.ts`,
    then run `bun run generate effects`. Masters go under
    `assets/source/audio/effects/` in LFS.
  - Run it through Bento's service proxy (`bento service proxy`, ElevenLabs)
    when the generator needs it. Credentials stay inside the CLI.
  - Make at least two takes per clip, then audition them in the sound
    workbench.
- **Assignment.** The `smoke_looks.<name>.sound` slot (slice 07) names the
  sound. Add the `audio.json` entry, plus every mirror the mix check requires:
  - `sound/tables.rs:253`;
  - the battle sounds list (`:391`);
  - `mix.rs:151`;
  - `packages/battle-audio/src/audioMix.ts:138`.
- **Trigger.** The effects publication emits a smoke pop for each cloud in
  `smoke_clouds` whose `popped` tick lies within `bloom_s` of the frame's tick
  and that wasn't voiced before. The pop is keyed by cloud id and placed at the
  cloud's centre. Mirror this in `packages/battle-audio/src/soundFrame.ts` if
  the browser sound frame has its own copy.
- **No synthesized fallback** (D16; none is required, `sound/catalog/check.rs:198`).

## What you can hear

The smoke lab and a skirmish, from both sides.

## Tests and checks

- **A sound-frame stream test:**
  - a side that sees the cloud at its pop plays the pop once, at the cloud;
  - a side that first sees it after the bloom plays nothing;
  - an own pop always plays.
- The sound vectors and the battle-audio checks, `UPDATE_SOUNDS=1` for the new
  entries only.
- **Listening:** audition in the sound workbench, then in the lab. Record the
  chosen takes in `choices.md`.

## Delegated

You may choose the prompt wording, take selection, gain within the mix check,
and clip crops.

## Must stay green

- Every other sound vector.
- The hearing-cue publication.
- The native client, which refuses missing LFS takes. Commit the takes via LFS.

## Feedback that would change this slice

If the user dislikes a take, regenerate or recrop it. If they want unseen pops
to be heard, that is a new hearing-cue category, which changes the publication
layout again; ask the user first.
