# Cinematic Capture Mode

Record marketing clips from battles with scripted camera paths and full in-game audio.

## Quick Start

```bash
# Record menu backdrop with default settings (1920x1080, 60fps, audio on)
bun run --cwd web capture

# Record at 4K with ground-level camera
bun run --cwd web capture --preset=ground-level --width=3840 --height=2160

# Record skirmish-style battle
bun run --cwd web capture --scene=skirmish

# List all options
bun run --cwd web capture --help
```

Output goes to `throwaway/capture/` as MP4 (H.264 + AAC). Use `--webm` for WebM output.

### Prerequisites

On a fresh worktree, ensure these are ready before running capture:

```bash
# Build the mechanics validator (required for map loading)
bun run build:mechanics

# Pull LFS assets (sounds, models)
git lfs pull
```

The CLI will detect missing prerequisites and print what's needed.

## CLI Options

| Option            | Default            | Description                |
| ----------------- | ------------------ | -------------------------- |
| `--preset=<name>` | default            | Camera preset              |
| `--width=<n>`     | 1920               | Output width in pixels     |
| `--height=<n>`    | 1080               | Output height in pixels    |
| `--fps=<n>`       | 60                 | Frame rate                 |
| `--no-audio`      | —                  | Disable audio recording    |
| `--no-cards`      | —                  | Hide info cards            |
| `--webm`          | —                  | Output WebM instead of MP4 |
| `--output=<path>` | throwaway/capture/ | Output directory           |
| `--scene=<name>`  | menu               | Scene: menu or skirmish    |

## URL Parameters

Access capture mode directly in the browser:

```
# Menu backdrop
http://localhost:5173/labs/capture?capture=default

# Skirmish battle
http://localhost:5173/labs/capture-skirmish?capture=default
```

| Parameter        | Default | Description                          |
| ---------------- | ------- | ------------------------------------ |
| `capture`        | —       | Camera preset name (required)        |
| `capture-width`  | 1920    | Output width in pixels               |
| `capture-height` | 1080    | Output height in pixels              |
| `capture-fps`    | 60      | Target frame rate                    |
| `capture-audio`  | true    | Include audio in recording           |
| `capture-cards`  | true    | Show info cards (with zoom fade)     |
| `capture-max`    | 120     | Maximum recording duration (seconds) |

## Camera Presets

Built-in presets provide common cinematic shots:

| Preset         | Duration | Description                                                  |
| -------------- | -------- | ------------------------------------------------------------ |
| `default`      | ~37s     | Wide top-down view transitioning to ground-level tank-follow |
| `top-down`     | 12s      | High-altitude strategic overview                             |
| `ground-level` | 15s      | Low-angle dramatic shot following a unit                     |
| `orbit`        | 20s      | Camera orbits around a central point                         |

## Adding Custom Presets

Edit `apps/battle-lab/src/capture/captureConfig.ts` to add new presets:

```typescript
export const CAPTURE_PRESETS: Record<string, CapturePreset> = {
  // ... existing presets ...

  "my-custom": {
    name: "My Custom Shot",
    description: "Description of the shot",
    reel: {
      fade_s: 0.6, // Fade duration at cuts
      shots: [
        {
          seconds: 10, // Shot duration
          follow: 0, // Optional: unit ID to track
          from: {
            // Starting camera pose
            target: [x, y], // World position to look at
            distance: 50, // Distance from target (meters)
            yaw: -0.5, // Horizontal angle (radians)
            pitch: 0.2, // Vertical angle (radians)
          },
          to: {
            // Ending camera pose (interpolated)
            target: [x, y],
            distance: 40,
            yaw: -0.3,
            pitch: 0.15,
          },
        },
        // Additional shots...
      ],
    },
  },
};
```

### Camera Pose Parameters

- **target**: `[x, y]` world coordinates the camera looks at
- **distance**: Camera distance from target in meters (lower = closer)
- **yaw**: Horizontal rotation in radians (0 = north, π/2 = east)
- **pitch**: Vertical angle in radians (0 = horizontal, π/2 = top-down)
- **follow**: Optional unit ID to track (offsets target relative to unit)

### Shot Interpolation

The camera smoothly interpolates between `from` and `to` over the shot's duration.
When following a unit, the camera tracks it with a slight lag for cinematic smoothness.

## Audio

Audio is enabled by default. The capture system:

- Taps the battle's audio output via `BattleAudio.createCaptureTap()`
- Records full in-game sound: weapons, vehicles, ambience, voices
- Uses zero-cost implementation when capture is off

Use `--no-audio` to disable audio recording.

## Scenes

Two scenes are available:

### Menu Backdrop (`--scene=menu`)

Records the menu backdrop battle (market-town map with encounter units).
This is the same battle that plays behind the main menu.

### Skirmish (`--scene=skirmish`)

Records the street test map with the "advance" encounter.
Blue forces advance through a street against red defenders.

## Output Format

**Default (MP4):**

- Container: MP4
- Video: H.264 (CRF 18, constant 60fps)
- Audio: AAC (192kbps)
- Requires ffmpeg for transcoding

**WebM (`--webm`):**

- Container: WebM
- Video: VP9 (16 Mbps)
- Audio: Opus (192kbps)
- Direct from MediaRecorder (no ffmpeg needed)

## Technical Notes

### Zero-Cost When Off

The audio capture tap (`WebAudioSink.createCaptureTap()`) is only created when
capture mode is active. Normal gameplay has no overhead from the capture system.

### Resolution and Performance

Recording at higher resolutions may affect real-time performance. If frames drop:

- Lower the resolution
- Use a preset with fewer close-up shots
- Ensure the GPU can handle the scene at the target resolution

### ffmpeg Transcoding

The CLI auto-transcodes WebM to MP4 using ffmpeg. If ffmpeg is not found,
the output remains WebM with a warning. Install ffmpeg:

```bash
# macOS
brew install ffmpeg

# Ubuntu/Debian
sudo apt install ffmpeg

# Windows
choco install ffmpeg
```

## Development

The capture system consists of:

- `captureConfig.ts` - Settings, presets, and URL parameter parsing
- `CinematicCapture.tsx` - Recording hooks and overlay UI
- `routes/capture.tsx` - Menu backdrop capture route
- `routes/captureSkirmish.tsx` - Skirmish capture route
- `web/capture.mjs` - CLI wrapper for automation
- `web/scenes/capture.mjs` - Scene test for menu capture
- `web/scenes/capture-skirmish.mjs` - Scene test for skirmish capture
- `packages/battle-audio/src/webAudioSink.ts` - Audio capture tap
- `packages/battle-audio/src/battleAudio.ts` - Battle audio tap wrapper
