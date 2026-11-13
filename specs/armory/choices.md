# Armory: choices

## Slice 07

- **Fade: 0.5 s, in and out.** A dropped loop fades over 0.5 s, and a new one
  ramps in over the same time (its `attack`). Why: a unit switch reads as a
  quick crossfade, well under the menu's 1.2 s bed fade, which is a
  screen-change pace. Alternative: reuse `MENU_FADE_S`, which drags on a list
  sweep. *Provisional*: slice 11 hears it.
- **Id range: open-ended, from 16 up.** The sink keys voices in a map, so the
  range needs no cap; each new show voice takes the next id. The bed (1) and
  the cue (3) sit below. Alternative: a fixed block of N ids, which adds a
  "too many loops" case that no unit has. *Sound.*
- **The bed restart clears by id.** `WebAudioSink.clear` takes an optional
  id filter, and the bed restart clears everything but show voices. Why: the
  restart has to cut the previous bed while it is still fading, and a stopped
  voice has already left the id map, so stopping slot 1 cannot reach it.
  Alternative: a second sink for show loops, which would need its own master,
  limiter and volume wiring. *Sound.*
- **Leaving the menu screen forgets the asked-for loops; mute keeps them.**
  `loading` fades them out and `other` cuts them with the rest of the menu
  mix. Returning to the menu plays nothing until a page asks again. Why: the
  loops belong to the page that asked, and that page is gone. Mute is a
  setting, so unmuting resumes what was asked for. *Sound.*
- **Readiness has one owner.** `SoundBank.prepared(name)` reports whether
  `get` can return a sound. `AppAudio` starts a loop only once its sound is
  prepared, and asks the bank for the missing ones. The bank already joins
  duplicate preparations. Alternative: a ready set in `AppAudio`, which
  duplicates the bank's own state. *Sound.*
- **Show loops ride the `units` bus with no low-pass, no reverb and no
  position.** The spec names the bus. The rest matches the bed: the sound is
  heard at the camera, not placed in a world. *Sound.*
- **Loops are keyed by sound name.** If a call names the same sound twice,
  the last entry wins. *Sound.*
