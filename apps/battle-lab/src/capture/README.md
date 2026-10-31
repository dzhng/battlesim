# Capture

Scripted battle videos for marketing, filmed from a JSON file. An agent writes
the file and one command turns it into MP4s. Nobody drives this by hand, and
there is no UI. The design and its reasons live in
[the capture spec](../../../../specs/done/capture/README.md).

## Two files, two owners

- An **encounter** says what happens: units, scripted orders and events, and
  stat `overrides`. It lives at `fixtures/maps/<map>/encounters/<name>.json`
  ([saved maps](../../../../fixtures/README.md#saved-maps)). Making a unit
  tougher, retiming an attack or moving a tank for a better shot is encounter
  work.
- A **capture** says how it is filmed: camera and scenes only. It lives at
  `fixtures/captures/<name>.json`, and the CLI refuses a file anywhere else.
  It never carries units, orders or stats.

Only maps whose `meta.json` category is `capture` are filmable. A `test` map
belongs to the tests and stays as it is. To film one, copy its folder under a
new id, set the category to `capture`, and keep or write the encounters you need
(the map's `meta.json` lists them, in step with its `encounters/` folder).
Capture maps change as often as a film needs. A capture may field the capture
unit set's own units (`capture_*`, [catalog sets](../../../../fixtures/README.md)).

The main menu's backdrop is the capture
[`fixtures/captures/menu.json`](../../../../fixtures/captures/menu.json), played
live by the same [`ScenePlayer`](ScenePlayer.tsx) that the capture route
records. Menu and videos are one system: a fix to one is a fix to both.

## The format

[`capture.ts`](capture.ts) owns the format. `validateCapture` refuses an unknown
field, a non-capture map or a bad value, naming the field. The other files in
`fixtures/captures/` are working examples.

```jsonc
{
  "scenes": [
    // played in order; several are joined into one MP4, dipping through black between them
    {
      "map": "market-town", // a capture map's folder name
      "encounter": "menu", // file stem under that map's encounters/
      "seed": 1, // the battle's seed; the music shuffles by it too
      "warm_s": 3, // seconds simulated before the first frame
      "reel": {
        "fade_s": 0.6,
        "shots": [
          /* … */
        ],
      },
    },
  ],
}
```

`fade_s` is the dip to black at each end of every shot, so the film opens from
black, cuts through black and ends on black. Every shot must last at least
`2 × fade_s`. Use `0` for no dips.

A **shot** is one camera move from `from` to `to`:

| Field          | Meaning                                                                                                                                                                                                                                                                                                                                                                 |
| -------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `seconds`      | The shot's length, dips included.                                                                                                                                                                                                                                                                                                                                       |
| `from`, `to`   | Poses: `target` `[x, y]` on the ground in metres, `distance` from it in metres, `yaw` and `pitch` in radians. Pitch is the angle down to the target: about 0.1 skims the ground, 1.2 looks almost straight down. Yaw 0 → 2π is one full orbit.                                                                                                                          |
| `follow`       | Optional. Tracks one unit: its index in the encounter's `units`, which must be a `blue` unit. The film is blue's view, so it can't follow what blue can't see. `target` is then an offset in metres from the unit, and the camera glides after it. When the unit dies, the camera holds where it last stood. Without `follow`, `target` is a world position on the map. |
| `ease`         | Optional. `"linear"` (default) or `"cubic"`, an ease-in-out.                                                                                                                                                                                                                                                                                                            |
| `logDistance`  | Optional. Interpolates distance in log space, so a long zoom feels even rather than rushing at the end.                                                                                                                                                                                                                                                                 |
| `zoomDuration` | Optional. The move takes this many seconds and then holds `to` for the rest of the shot. Defaults to `seconds`.                                                                                                                                                                                                                                                         |

A pose may come closer than a player can zoom: the film's camera rig reaches in
to the closest distance in the capture.

The battle runs live and is not scripted by the capture. A shot shows whatever
the encounter makes happen at that moment. Only `menu.json` is held to a
recorded event log (`crates/sim/tests/menu_reel.rs`, see
[the backdrop's rules](../../../../fixtures/README.md)). Any other capture can
retime after a rule change, so watch it again before reusing the footage. The
same test checks every capture file's maps and `follow` indices, and is the
fastest check after editing one:
`cargo test -p sim --test sim every_capture`.

## Worked example

Goal: a 14-second clip of the opening tank push on Market Town, using its
existing `menu` encounter.

1. **Pick the battle.** `market-town` is a capture map, and its `menu`
   encounter opens on blue's tanks. Unit 0 is the lead `capture_us_tank`
   (`side: "blue"`), so it can be followed.
2. **Frame the first shot.** Follow unit 0 from 45 m behind and slightly to the
   side, low (pitch 0.16), and drift in to 40 m while turning 0.3 rad:

   ```json
   {
     "seconds": 8,
     "follow": 0,
     "from": { "target": [-1, 8], "distance": 45, "yaw": -0.126, "pitch": 0.16 },
     "to": { "target": [-1, 8], "distance": 39.6, "yaw": -0.426, "pitch": 0.147 }
   }
   ```

3. **Add a wide shot.** A world-fixed view over the town for 6 s, with no
   `follow`, so `target` is a map position:

   ```json
   {
     "seconds": 6,
     "from": { "target": [3585, 1940], "distance": 62, "yaw": -1.45, "pitch": 0.14 },
     "to": { "target": [3575, 1935], "distance": 56, "yaw": -1.6, "pitch": 0.13 }
   }
   ```

4. **Wrap it** in a scene and save it as `fixtures/captures/push.json`:

   ```json
   {
     "scenes": [
       {
         "map": "market-town",
         "encounter": "menu",
         "seed": 1,
         "warm_s": 3,
         "reel": {
           "fade_s": 0.6,
           "shots": [
             /* step 2, step 3 */
           ]
         }
       }
     ]
   }
   ```

5. **Check it** with the test above, then **preview** it live on the dev server
   (`bun run dev`) at `/labs/capture?capture=push&scene=0`. That is the same
   page the CLI records, without recording. Adjust the poses, or edit the
   encounter if the action is wrong, until the shots read.
6. **Film it** with `bun run --cwd web capture fixtures/captures/push.json`.
   Watch the MP4 end to end and read the report.

## Filming

```bash
bun run --cwd web capture fixtures/captures/<name>.json [--height=1080|1440] [--fps=<n>] [--no-cards] [--output=<dir>]
```

The usage text in [`web/capture/args.mjs`](../../../../web/capture/args.mjs)
owns the flags. Output is 16:9, 1080p by default and 1440p at most; 60 fps by
default; unit info cards on unless `--no-cards` (as in the game, they fade out
as the camera closes in, so a close shot is clean); the colour grade is always on.

Each scene plays on the capture route ([`routes/capture.tsx`](../routes/capture.tsx))
in a headful Chromium window, which records its stage, cards and every sound
(music included) with tab capture ([`tabRecorder.ts`](tabRecorder.ts)), in
real time. It writes to `throwaway/capture/` by default:

- `<name>-scene<i>.mp4`: one constant-rate H.264/AAC MP4 per scene, next to the
  WebM it was made from;
- `<name>.mp4`: all scenes joined, dipping through black for 0.5 s between them, when there are several;
- `<name>.report.json`: the capture file, and for each output its length,
  average fps, worst frame gap and whether it has audio, plus warnings.
  ffprobe measures all of them on the recorded WebM; the page hands over only
  the chunks and `{seconds}` (or `{error}`).

Recording is wall clock, so a slow machine records choppier, never slower or
out of sync. The report warns when a scene averages below 90% of the target
fps. At 1440p the renderer, not the recorder, is the usual limit.

A failure exits non-zero with a named error
([`web/capture/recording.mjs`](../../../../web/capture/recording.mjs)):

- `TabCaptureNotStarted`: nothing was recorded. Chrome showed a permission
  prompt or ignored the auto-accept flag;
- `TabCaptureTimeout`: the recording started but never finished;
- `TabCaptureFailed`: the page reported an error, such as a capture that fails
  validation or a battle that can't start;
- `TabCaptureNoAudio`, `TabCaptureEmpty`: the WebM has no sound or no video;
- `FfmpegMissing`: `ffmpeg` or `ffprobe` is not on `PATH`.

### Machine

Recording needs the user's Mac and a visible Chrome window: WebGPU and tab
capture are proven there, and never headless or in CI. The browser launches
with `--auto-accept-this-tab-capture`, which skips the permission prompt. If a
Chrome update breaks that flag, the run fails with `TabCaptureNotStarted`
rather than hanging. The CLI takes the repository's GPU lock while it records,
so a run waits for other rendering jobs.

The `capture` browser scene (`bun run --cwd web scene -- capture`) proves the
recording path on its own, through the CLI's own receiver, and reports the
frame rate as the CLI measures it. It is a tool, so the full verify skips it.

## HDR Capture

For HDR gameplay clips suitable for X/iPhone playback, use the native client's
HDR output path. This produces real HDR (HLG, BT.2020, 10-bit HEVC) where
highlights (muzzle flashes, fires, explosions, bloom, sun glints, tracers) can
exceed SDR white while midtones match the SDR look.

### Prerequisites

1. Build the native client in release mode:

   ```bash
   cargo build --release -p native-client
   ```

2. Have a native parity run recorded. Run `bun run --cwd web capture:parity`
   (or the native-parity-capture command) to produce a replayable run under
   `throwaway/parity/`. The standard `bun run --cwd web capture` writes MP4s
   directly and does not produce replayable runs.

3. `ffmpeg` and `ffprobe` must be on PATH.

### Recording HDR frames

Use the native client with `--hdr --all-frames` to replay a run with HDR output:

```bash
./target/release/native-client \
  --run throwaway/parity/<run-dir> \
  --scene <map-name> \
  --out throwaway/hdr-frames \
  --hdr \
  --all-frames \
  --sound throwaway/hdr-frames/sound.wav \
  --window offscreen \
  --pace 0
```

The `--run` argument is the parent folder containing the scene directories;
the client appends `<scene>/` to find the actual run data.

Additional flags:
- `--from-ms <n>`: Only render frames from this millisecond (default: start)
- `--to-ms <n>`: Only render frames up to this millisecond (default: end)
- `--no-music`: Disable music during sound rendering

This produces:
- EXR frames (HLG-encoded BT.2020 half-float) in the output directory
- A WAV file with rendered audio

### Encoding to HLG

Use the HDR capture script to orchestrate the full pipeline:

```bash
bun run --cwd web hdr-capture <run-dir> <scene> [--fps=60] [--h264] [--output=<dir>]
```

The `<run-dir>` is the parent folder of scene directories (same as `--run`).

Flags:
- `--fps=<n>`: Target frame rate (default 60)
- `--h264`: Also emit an H.264 HLG variant (X serves this format back)
- `--output=<dir>`: Output directory (default `throwaway/hdr-capture/`)
- `--keep-frames`: Keep intermediate EXR frames after encoding
- `--from-ms=<n>`: Start time in milliseconds
- `--to-ms=<n>`: End time in milliseconds
- `--no-music`: Disable music during sound rendering

Output files:
- `<scene>-hdr.mp4`: HEVC Main10 HLG video
- `<scene>-hdr-h264.mp4`: H.264 High10 HLG variant (if `--h264`)
- `<scene>-hdr.report.json`: Encoding metadata and verification results

### Verifying HDR metadata

The script verifies the output with ffprobe. The expected metadata:

```
codec_name: hevc (or h264)
profile: Main 10 (or High 10)
pix_fmt: yuv420p10le
color_primaries: bt2020
color_space: bt2020nc
color_transfer: arib-std-b67
color_range: tv
```

You can manually verify with:

```bash
ffprobe -v error -select_streams v:0 \
  -show_entries stream=codec_name,profile,pix_fmt,color_primaries,color_space,color_transfer,color_range \
  -of json output.mp4
```

### HDR tonemap design

The HDR path uses a modified tonemap that:
- Keeps midtones identical to SDR (same color grade + AgX-like shape)
- Places diffuse/reference white around 203 nits (HLG 0.75)
- Rolls highlights up to ~1000 nits (about 5× headroom, HLG 1.0)
- Desaturates very bright colors toward white like AgX
- Converts from Rec.709/sRGB working space to BT.2020 primaries
- Applies HLG OETF directly in the shader (not ffmpeg)

UI overlays and fog-of-war rim are drawn into the HDR target at reference white.

### Why HLG

HLG (Hybrid Log-Gamma) is chosen over PQ because:
- SDR backward compatibility: HLG degrades gracefully on SDR displays
- iPhone/X native support: Both platforms recognize HLG metadata
- Simpler mastering: No absolute brightness targets to hit

### Manual encoding

For custom encoding, use the EXR frames directly with ffmpeg. Since the shader
applies HLG OETF, the frames are already in HLG space:

```bash
ffmpeg -y -framerate 60 -i frames/frame-%06d.exr -i sound.wav \
  -vf "scale=1920:1080:out_color_matrix=bt2020:out_range=tv,pad=1920:1080:(ow-iw)/2:(oh-ih)/2,setsar=1,setparams=color_primaries=bt2020:color_trc=arib-std-b67:colorspace=bt2020nc" \
  -c:v libx265 -preset slow -crf 18 -profile:v main10 \
  -pix_fmt yuv420p10le \
  -color_primaries bt2020 -colorspace bt2020nc -color_trc arib-std-b67 \
  -color_range tv -tag:v hvc1 \
  -x265-params repeat-headers=1 \
  -c:a aac -b:a 192k -ar 48000 \
  -movflags +faststart \
  output-hdr.mp4
```
