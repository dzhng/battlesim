// The native parity control run (`_parityRun.mjs`), of the menu's reel.
// PARITY_CAPTURE=<name> films another reel (`?capture=`: `water`, the water
// scene); PARITY_SCENES=<map,…> films only those scenes (`?scenes=`);
// PARITY_FOG=1 films them with the side's fog of war (`?fog=1`, the
// manifest's `fog` layer). PARITY_COUNTS=1 also counts what chosen frames
// drew, on tick boundaries and between (`?counts=1`, the manifest's
// `frameCounts`): the committed fixture `native-client`'s replay test counts
// against. Run it under the GPU lock.
import { filmParityRun } from "./_parityRun.mjs";

export async function run(ctx) {
  const { PARITY_CAPTURE: capture, PARITY_SCENES: scenes } = process.env;
  const counts = process.env.PARITY_COUNTS === "1";
  const url = new URL(ctx.url);
  if (capture) url.searchParams.set("capture", capture);
  if (scenes) url.searchParams.set("scenes", scenes);
  if (process.env.PARITY_FOG === "1") url.searchParams.set("fog", "1");
  if (counts) url.searchParams.set("counts", "1");
  await filmParityRun(ctx, url.href, { counts });
}
