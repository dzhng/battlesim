// The native parity control run (`_parityRun.mjs`), of the menu's reel.
// PARITY_CAPTURE=<name> films another reel (`?capture=`: `water`, the water
// scene); PARITY_SCENES=<map,…> films only those scenes (`?scenes=`);
// PARITY_FOG=1 films them with the side's fog of war (`?fog=1`, the
// manifest's `fog` layer). The battle view's indicators are filmed and
// recorded (`?indicators=1`: `overlays.bin`, the manifest's `overlays`
// layer) unless PARITY_INDICATORS=0, independently of the fog.
// PARITY_COUNTS=1 also counts what chosen frames drew, on tick boundaries
// and between (`?counts=1`, the manifest's `frameCounts`): the committed
// fixture `native-client`'s replay test counts against. Run it under the
// GPU lock.
import { filmParityRun } from "./_parityRun.mjs";

/** The graphics test's URL from the scene's `base` URL and the `PARITY_*`
 *  variables in `env`. */
export function parityUrl(base, env) {
  const url = new URL(base);
  if (env.PARITY_CAPTURE) url.searchParams.set("capture", env.PARITY_CAPTURE);
  if (env.PARITY_SCENES) url.searchParams.set("scenes", env.PARITY_SCENES);
  if (env.PARITY_FOG === "1") url.searchParams.set("fog", "1");
  if (env.PARITY_INDICATORS !== "0") url.searchParams.set("indicators", "1");
  if (env.PARITY_COUNTS === "1") url.searchParams.set("counts", "1");
  return url.href;
}

export async function run(ctx) {
  const counts = process.env.PARITY_COUNTS === "1";
  await filmParityRun(ctx, parityUrl(ctx.url, process.env), { counts });
}
