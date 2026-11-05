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

## HDR clips

`bun run --cwd web hdr-capture <run-dir> <scene>` makes an HDR clip of a
recorded battle, for posting where phones show HDR (X on an iPhone). It is a
second path, not a mode of the browser capture: the browser can't encode HDR
video, so the release native client replays a native parity run (`bun run --cwd
web scene -- native-parity-capture` writes one under `throwaway/native-parity/`)
at a fixed frame rate, with the replay's own sound
([native client](../../../../native/client/README.md)). The script
([`web/capture/hdr-capture.mjs`](../../../../web/capture/hdr-capture.mjs)) owns
its flags. Clip frames are on the reel's clock, not the wall's, so a slow
machine is slower to finish, never choppier, and the battle in them is on it
too: units move at even steps with the camera rather than at the jittery times
the page drew them.

**The look is encoded once, in the renderer.** A runtime built with
`hdr_output` sends its look to a half-float target instead of the display: the
same grade and AgX midtones as the SDR look, with highlights (muzzle flashes,
fires, sun glints) rolled up past white toward the HLG peak and desaturated
toward white as AgX does. The shader then applies the BT.2100 HLG encoding
(reference white at 203 nits, signal 0.75; a 1000-nit peak at 1.0) in BT.2020
primaries. The fog rim is drawn over it in the same signal. ffmpeg only
converts the frames to BT.2020 YUV and tags the stream. The script refuses a
first frame of one flat colour (nothing drawn) before encoding, and a stream
whose tags say otherwise after
([`web/capture/hdr.mjs`](../../../../web/capture/hdr.mjs)).

The clip is 8-bit H.264 High by default, because X's web uploader hangs on
HEVC; `--hevc` writes 10-bit HEVC Main 10 instead. Both carry the same BT.2020
HLG tags.

**Indicators are the game's, recorded.** The parity run films the battle
view's in-world indicators with the battle: contact glyphs (a remembered
contact's too, while it fades), aircraft marks, the zone, objectives, roads in
and the map's border, built by the live game's own builders from blue's own
view (`useBattleOverlay`). A hidden enemy has no mark, and a contact never
gets an identified unit's. The page records what it uploaded each frame
beside the capture (`overlays.bin`, the manifest's `overlays` layer); native
draws exactly that, the ground marks as paint in the world and the rest laid
over the look as an HLG signal at reference white, as the fog rim is.
`PARITY_INDICATORS=0` films a run without them.

**Cards are the game's, recorded.** The parity page shows the battle view's
own unit info cards (`battleReadouts`): blue's units in cyan, the enemies it
has identified in red, its contacts (a remembered one fading out as it
retires, still saying what blue last knew of it), laid out by the live
readout layer, fading as the camera closes from 90 m to nothing at 48 m. A
hidden enemy has no card. Each frame the page reads back what that layer
placed, each card's backing, words, icons, place, opacity and leader, and
records it with the pixels of its glyphs and icons in the page's own fonts
(`cards.bin`, the manifest's `cards` layer;
[`cardCapture.ts`](../../../../web/src/battle/benchmark/cardCapture.ts) owns
the format). Native draws those primitives over the look, after the fog and
the indicators, through the indicators' composite. `PARITY_CARDS=0` films a
run without them, and `PARITY_CARDS=open` films them open, as holding Space
opens them; at rest a card is compact (its name, its status marks and a
carrier's seats).

At playback, `--no-indicators`, `--no-cards` and `--no-fog` each leave out
one filmed layer; `--no-indicators --no-cards` makes a clean clip, with or
without the fog. `PARITY_VIEWPORT=1920x1080` films the run at the clip's
size. The `cards` reel (`PARITY_CAPTURE=cards`,
[`fixtures/captures/cards.json`](../../../../fixtures/captures/cards.json))
watches the menu battle's contacts from a commander's height, about 120 m,
then closes in through the fade.

HLG rather than PQ: an SDR screen shows an HLG stream acceptably without
tone-mapping metadata, and iPhone and X both recognize it.
