# Cleanup presentation-history control

Two full browser gates failed the exact cleanup byte assertion with a 2688-byte variation. Native ticks, effects, corpses, buffer counts, textures, workers and listeners matched. The original failing gates remain recorded; three uninstrumented/labelled focused runs passed by chance, so they did not establish a fix.

A read-only allocation probe recorded actual device buffer creation stacks and destruction. Drawing the final tick at the starting camera before the camera jump changes exactly two scenery near-tier buffers: 3072/3072 bytes become 3936/4896, totaling the same 2688-byte difference. Both variants return the same final frame data. The camera before the jump is [170,800,0], after it [400,820,0]; all battles reach tick 2460. The controlled four fights consistently retain 190972036 buffer bytes. This is retained view-dependent capacity, not evidence of a leaked allocation.

Fast-forward completion releases a decoded publication before React necessarily draws it. The cleanup harness now waits through the existing `presented` helper before moving the camera. That helper checks both the last drawn observation tick and its presentation clock. Every cycle therefore draws the same final starting-camera view before the same jump; the exact allocation assertions are unchanged. No renderer allocation policy, shader, tolerance or simulation rule changes.

The two probe snapshots and full actual allocation records preserve the cause experiment. The focused production cleanup scene independently passes all unchanged assertions; its actual exit is zero and full reset/remount totals are in `production-focused-green.log`. Final complete gates are rerunning.
