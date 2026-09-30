// Opt-in real-battle filmstrip: timing and tracer readability amid unit readouts.
import { writeFile } from "node:fs/promises";
import { openBattle, obs, advance, aim, snapshot } from "./_lab.mjs";
import { curvePitch, village } from "./_units.mjs";

export async function cadenceTour(ctx) {
  const page = await openBattle(ctx, {
    viewport: { width: 1920, height: 1080 },
    tick: 1140,
    grass: true,
  });
  const o = await obs(page);
  const squad = o.own.find((u) => u.kind === "rifle" && u.mounts.some((m) => m.target));
  if (!squad) throw new Error("cadence framing needs an engaged rifle squad");
  await aim(page, [squad.position[0] + 25, squad.position[1], squad.position[2]], {
    distance: 65,
    pitch: curvePitch(65),
    yaw: village.presentation.camera.default.yaw,
  });
  const frames = [];
  for (let i = 0; i < 24; i++) {
    if (i) await advance(page, 3);
    const current = await obs(page);
    const name = `cadence-${String(i).padStart(2, "0")}.png`;
    await snapshot(ctx, page, name);
    frames.push({
      tick: current.tick,
      file: name,
      rifles: current.projectiles
        .filter((p) => p.kind === "rifle")
        .map((p) => ({ soldier: p.shooterMember, path: p.path })),
    });
  }
  await writeFile(ctx.evidencePath("cadence.json"), JSON.stringify(frames, null, 2));
  ctx.check(
    "the real-battle cadence filmstrip contains rifle fire",
    frames.some((f) => f.rifles.length > 0),
  );
  await page.close();
}
