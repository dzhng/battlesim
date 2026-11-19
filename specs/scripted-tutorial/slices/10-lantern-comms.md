# 10 Lantern's comms panel and narration

**Unlocks:** the coach speaks. Lantern's portrait plays each step's line as a
radio transmission, and the takes come through the same judged voice pipeline
as unit voices.

**Inputs (frozen by the interview):**

- Voice: ElevenLabs shared library **Eryn**, `wa4sQVgbDDzUDEzJwch3`, model
  `eleven_v3`.
- Portrait: [assets/portraits/lantern.png](../assets/portraits/lantern.png).
- Script and review takes: [assets/narration/](../assets/narration/) (58
  lines, all passing speech-to-text).
- Animation: a still portrait with comms effects (an audio-driven signal bar, a
  little scan flicker), not lip-sync.

**Contract (C10):**

- **A narration set in the voice catalog.** `fixtures/voices.json` (or its
  narration section; the shape is decided against `voiceCatalog.ts`) gains a
  speaker `lantern` with its library `voice_id` and lines by id. Ids are the
  coach `line` ids. Text lives here once; [script.json](../assets/narration/script.json)
  is retired into it.
- **The same lock, verdicts and packs.** A take's raw master goes to
  `assets/source/audio/voices/narration/lantern/` with its provenance
  (generator, model, voice, seed, date). Its verdict carries the hash of the
  bytes heard. The game plays only judged takes. Import the review takes when
  their text is unchanged (they passed speech-to-text; judge them by ear in the
  sound workbench). Re-render only lines whose text changed.
- **Transport.** The generator (`packages/battle-audio/tools/generate/`) talks to
  ElevenLabs through the `bento` CLI (`bento service proxy … --service-id
  elevenlabs`), the way `packages/scene-assets/tools/skins/fal.mjs` already
  does. The Duet base URL and its config file go. Voice design isn't admitted by
  Bento's proxy and created voices expire after 24 h, so the design step stays
  unused for narration: a speaker with a library `voice_id` skips design.
  Record in the ledger whether the unit-voice design step still works through
  the old path or is retired.
- **Processing.** A radio profile from the existing processing profiles, so
  Lantern sounds like HQ on the net (pick by ear in the sound workbench).
- **Playback.** Battle audio plays a step's line once when the step appears. A
  line playing ducks unit chatter (the mix owner decides how much). A pause
  suspends it with everything else.
- **Panel.** `CoachPanel` becomes the comms panel:
  - the portrait (with a holo treatment built from existing colour roles, never
    a new hue);
  - the signal bar driven by the playing take's level;
  - the spoken line as a caption;
  - the card text with key glyphs under it;
  - Continue on `continue` steps.

  Load game-ui first. It must not read as a chat widget or a SaaS toast.
  Iterate it in isolation on a plain background in every permutation (speaking
  and silent, each gate kind, long and short card), then in the battle.

**Playable:** chapters 1–2 coached by Lantern's voice and portrait.

**Gates:**

- Voice catalog validation (`validateVoiceCatalog`) admits the narration set
  and refuses a line id the coach references that has no judged take (coach
  validator plus pack listing).
- `bun test` for the generator's transport (a mocked `bento` exec, as the
  skins' tests mock it).
- Panel shots in isolation and in battle at default and far zoom, over grass,
  road, fog and smoke. Compare-screenshots against 09's plain panel (variable:
  the panel reads as a radio transmission, not a widget). Screenshot-critique
  last. Show them inline (non-blocking), with one take embedded so the user
  hears the processed voice.

**Delegated:** the catalog shape for narration within `voiceCatalog.ts`'s
conventions, the portrait's crop and treatment, the signal bar's form.

**Must stay green:** voice catalog and pack tests, the sound workbench, existing
unit voice playback.

**Feedback that changes it:** the user wants lip-sync after all (a fal
lip-sync slice of its own); a different voice (re-render every line from the
catalog in one run).
