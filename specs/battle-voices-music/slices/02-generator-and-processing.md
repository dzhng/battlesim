# 02 — Generation tool and processing

**Contract this unlocks:** any line can be turned into a finished, served, verified clip. The steps are: generate a raw master → process it to the set's treatment → transcribe it → record it in `fixtures/voices.lock.json`. Everything is testable offline, and a real call is one command.

## Seam

**`packages/battle-audio/tools/generate/` (TypeScript, run with Bun):**
- **`elevenlabs.ts`** holds `ElevenLabsClient({ fetch, base, workspace, token })`, with the methods `tts`, `stt`, `designVoice`, `saveVoice` and `music`.
  - `BASE = "https://ctl.duet.so/v1/services/elevenlabs"` is the **only** place the proxy URL appears.
  - The token and workspace are read from `~/.duet/config.json`. Never print them.
- **`cacheKey.ts`:**
  - The generation key is sha256 of (text after `{callsign}` expansion, voice_id, model `eleven_v3`, voice settings, retake).
  - The processing key is sha256 of (raw sha, profile, bed sha, encoder settings).
- **`pool.ts`:**
  - Bounded concurrency: 8 TTS, 4 speech-to-text, 2 music.
  - Backoff with jitter on 429 and 5xx.
  - It stops on 401 or quota errors, or after N failures in a row.
- **`transcript.ts`:** `matches(script, transcript)` strips tags, normalises numbers ("two-one" = "21") and computes a word error rate. A dropped keyword (callsign, "sabot", "gunner") is flagged, and so is a take longer than its trigger's nominal `talk_s` (slice 11 owns those values; use the rule's defaults until then).
- **`lock.ts`:**
  - It reads and writes `fixtures/voices.lock.json`.
  - Files are written atomically: a temporary file, then rename.
  - The lock is flushed every 20 completions, so a killed run is simply rerun.
- **`cli.ts`** has these commands:
  - `generate [--set] [--only missing|bad|flagged] [--take K] [--dry-run] [--max-chars N]`
  - `process`
  - `check`
  - `report`

  It prints one progress line per take, e.g. `[312/5104] us_tank/lead/R02/stressed/1 ok 2.1s match 0.97`.

**`packages/battle-audio/tools/assets.py`** stays the only ffmpeg owner. It gains a `voice` subcommand and these voice profiles:

| Profile | Treatment |
|---|---|
| `voice-radio-light` | high-pass ~250 Hz, low-pass ~3.8 kHz, gentle compression, faint hiss, a tank engine bed at about −10 dB |
| `voice-dry` | none |
| `voice-earpiece` | high-pass ~250 Hz, low-pass ~4.5 kHz, faint pink hiss |
| `voice-intercom` | high-pass ~300 Hz, low-pass ~3.4 kHz, compression, a rotor bed |

These ffmpeg chains were tested during the walk; their outputs are in `assets/reference-voices/`:

```
radio-light (approved): highpass=f=250,lowpass=f=3800,acompressor=threshold=0.1:ratio=4:attack=5:release=60:makeup=2
                        + white hiss (anoisesrc amplitude 0.015, band 500–3000 Hz) at weight 0.3
                        + engine bed at volume 0.3
earpiece (approved):    highpass=f=250,lowpass=f=4500,acompressor=threshold=0.1:ratio=4:makeup=2
                        + pink hiss (amplitude 0.01, low-pass 3000) at weight 0.5
intercom (approved):    highpass=f=300,lowpass=f=3400,acompressor=threshold=0.1:ratio=6:attack=5:release=60:makeup=3
                        + white hiss band 500–3000 at weight 0.25 + rotor bed at volume 0.6
radio (heavier, "less" requested — do not use): adds asoftclip=type=tanh, ratio 8, hiss 0.4, engine 0.55
```

Other changes to `assets.py`:
- **A second input** for beds. Today filters take one input (`assets.py:41-46`, `:145`).
- **A loudness target** in place of the fixed-peak normalisation at `:162-165`, for voice profiles only.
- **Mono MP3 at about 40 kbps.**

**Beds:**
- Generated once with `/v1/sound-generation`.
- Stored as raw masters under `assets/source/audio/voices/beds/`.
- Their provenance is `Generated`.

**Paths:**
- **Raw masters:** `assets/source/audio/voices/<set>/<voice>-<trigger>-<intensity>-<take>.<genkey8>.mp3`. These are LFS; `assets/source/**` is already LFS.
- **Served:** `assets/runtime/audio/voices/<set>/<voice>-<trigger>-<intensity>-<take>.<sha8>.mp3`. These are LFS through `assets/runtime/audio/**`, and Vite's `publicDir` serves them at `/audio/voices/…`.
- **`voiceUrl(set, take)`** in battle-audio is the only builder of that URL.

**Check:**
- `check` verifies hashes (raw sha → served sha recorded in the lock).
- It does **not** re-render and compare bytes, because MP3 encoding differs between ffmpeg versions. The existing `assets.py check` behaviour at `:200` stays as it is for WAV clips.

## What the human hears

The four saved T1 voices, each through every profile. This is one `generate --set us_tank,ru_rifle,us_recon,uk_pilot --only R2` run (about 32 calls). The output opens as a page the user can play.

## Verification

- **Tool tests** with a fake `fetch`: a cache hit makes no call; a run resumes after a kill mid-batch; a mismatch is flagged; a retake changes the generation key; a profile change re-processes but doesn't regenerate.
- **`assets.py voice`** on a fixture WAV gives mono MP3 near the target loudness, checked with `ffprobe`.
- **Serving:** `curl -I localhost:<port>/audio/voices/us_tank/…mp3` returns 200 under the dev server.

## Delegated to you
- module internals;
- the exact filter constants, within the T2 descriptions (the user judges by ear);
- the WER threshold.

## Must stay green
- `assets.py check` for existing WAV clips;
- `web/tests/soundCatalog.test.ts`.

## Feedback that would change this slice
- The user says a profile is too heavy or too light: tune its constants. Only re-processing runs, and no credits are spent.

## Checkpoint (non-blocking)
Open the profile page and wait about 5 minutes. With no reply, keep the T2 references' settings and record that.
