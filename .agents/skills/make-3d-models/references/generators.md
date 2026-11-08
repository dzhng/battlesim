# Image-to-3D generators

## Choosing

Pick by looking, not by reputation. Send the same input to more than one
generator, then compare the results side by side under the same light. Put a
clay or matcap render beside the textured one: it shows which detail is real
geometry and which is painted on. Write down the pick and a one-line why.

Measured so far:

| Generator | What we saw |
|---|---|
| Hunyuan3D 3.1 Pro (fal, geometry only) | Smoothest hard surfaces for airframes. Takes four views (front, back, top, bottom). Refuses a subject filling less than half the image. |
| Meshy | Wrinkles at the scale of a panel, even before decimation. Added a second nozzle to a single-engine jet. Wins small props with printed labels. |
| Tripo | Wins big hard-surface props where real geometry (knobs, bezels, dents) matters. Raw output is about 2M triangles. |
| Rodin | Softer than Hunyuan on the same airframe. |

The Meshy and Tripo verdicts come from about 150 props built with
[anykit](https://github.com/apekshik/anykit) (MIT), whose agent guide this
file draws on.

## Inputs

- **One object per image**, whole and centred, filling the frame, on a plain
  light background under soft, even light.
- **One photo means guessed proportions.** Give every side the generator
  accepts.
- **Each view is scaled to fill its image.** A multi-view generator stretches
  each view, so mixing a nose-on view with a side view reads span as length.
  Use only views whose long side is the same axis.
- **Feed renders of the measured model, not the drawing.** Fed a three-view
  directly, a generator gets the proportions right, but it reads panel lines
  as raised geometry and turns wheel circles into floating discs.

## What every generator gets wrong

Expect all of these and check for each:

- glass baked opaque;
- screens faked into the texture;
- depth invented for boxy things;
- real brands put on labels;
- speckled flat panels;
- one-sided details mirrored;
- neighbours fused (rails into tips, two nozzles from one).

Their UVs and colours paint unevenly, so strip them and paint with the game's
own materials.

## Output

- **Find the axes from the geometry.** Facing differs by generator: Tripo's
  front is +X, and its node can carry an extra 20–30° turn copied from the
  photo's three-quarter view; Meshy's is −Y. Find the length as the axis the
  fin or tip reaches furthest along, relative to the extent, then confirm on
  a render.
- **Ask for a face limit at generation.** Decimation is cheap only on small
  meshes.
- **Register before you seat anything.** Measure the skin's real lengths
  (belly, top, span) and fit it to the model, then place fittings on the
  skin's own surface.

## Running jobs

- **Retry transient failures.** About 10–20% fail (504s, connection resets,
  timeouts after about 10 minutes), so re-run the same ids.
- **Make re-runs free.** Skip outputs that already exist and resume a job
  left in flight. Never start a second pass over ids still running in another
  shell.
- **Pay once.** Pin each result with its hash, request id, inputs, the
  generator's terms and a note on what the inputs were drawn from, so it is
  reproducible without paying again. Log what each call cost; a call costs
  about $0.30–0.80.
- **Read the terms before committing output** to a public repo: commercial
  use and who owns the output. Record them in the pin.
- **Never print or commit the key.** Read it fresh inside the command that
  uses it.
