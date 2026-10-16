# Cinematic Capture Mode

Record marketing clips from the menu backdrop battle with scripted camera paths.

## Quick Start

```bash
# Record with default settings (1920x1080, 60fps, default camera path)
pnpm --filter battle-web capture

# Record at 4K with ground-level camera
pnpm --filter battle-web capture --preset=ground-level --width=3840 --height=2160

# List available presets
pnpm --filter battle-web capture --help
```

Output goes to `throwaway/capture/` as WebM (VP9 + Opus).

## URL Parameters

You can also access capture mode directly in the browser:

```
http://localhost:5173/labs/capture?capture=<preset>&capture-width=1920&capture-height=1080
```

| Parameter | Default | Description |
|-----------|---------|-------------|
| `capture` | — | Camera preset name (required to enable capture) |
| `capture-width` | 1920 | Output width in pixels |
| `capture-height` | 1080 | Output height in pixels |
| `capture-fps` | 60 | Target frame rate |
| `capture-audio` | true | Include audio in recording |
| `capture-cards` | true | Show info cards (with zoom fade) |
| `capture-max` | 120 | Maximum recording duration (seconds) |

## Camera Presets

Built-in presets provide common cinematic shots:

| Preset | Description |
|--------|-------------|
| `default` | Wide top-down view transitioning to ground-level tank-follow |
| `top-down` | High-altitude strategic overview |
| `ground-level` | Low-angle dramatic shot following a unit |
| `orbit` | Camera orbits around a central point |

## Adding Custom Presets

Edit `apps/battle-lab/src/capture/captureConfig.ts` to add new presets:

```typescript
export const CAPTURE_PRESETS: Record<string, CapturePreset> = {
  // ... existing presets ...
  
  "my-custom": {
    name: "My Custom Shot",
    description: "Description of the shot",
    reel: {
      fade_s: 0.6,  // Fade duration at cuts
      shots: [
        {
          seconds: 10,           // Shot duration
          follow: 0,             // Optional: unit ID to track
          from: {                // Starting camera pose
            target: [x, y],      // World position to look at
            distance: 50,        // Distance from target (meters)
            yaw: -0.5,           // Horizontal angle (radians)
            pitch: 0.2,          // Vertical angle (radians)
          },
          to: {                  // Ending camera pose (interpolated)
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

## Technical Notes

### Output Format

- **Container**: WebM
- **Video**: VP9 (16 Mbps)
- **Audio**: Opus (when enabled)

The browser's MediaRecorder API produces WebM. For MP4/H.264 output, you can
transcode with ffmpeg:

```bash
ffmpeg -i capture-default-2024-01-01.webm -c:v libx264 -crf 18 -c:a aac output.mp4
```

### Zero-Cost When Off

The audio capture tap (`WebAudioSink.createCaptureTap()`) is only created when
capture mode is active. Normal gameplay has no overhead.

### Resolution and Performance

Recording at higher resolutions may affect real-time performance. If frames drop:
- Lower the resolution
- Use a preset with fewer close-up shots
- Ensure the GPU can handle the scene at the target resolution

## Development

The capture system consists of:

- `captureConfig.ts` - Settings, presets, and URL parameter parsing
- `CinematicCapture.tsx` - Recording hooks and overlay UI
- `routes/capture.tsx` - Dedicated capture route
- `web/capture.mjs` - CLI wrapper for automation
- `web/scenes/capture.mjs` - Scene test for CI
