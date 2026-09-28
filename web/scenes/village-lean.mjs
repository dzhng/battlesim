// Soldiers lean out round tall cover. On the village's
// ground, a blue squad at rest just inside the west wood trades fire with a
// red squad in the open. The scene steps the fight until a blue soldier is
// out on his lean, and shoots him at the ground camera (`lean-out`), then
// again once he has tucked back in (`tucked`), each also HUD-free.
import { readFile } from "node:fs/promises";
import { lab, obs, advance, snapshot } from "./_lab.mjs";

const village = JSON.parse(
  await readFile(new URL("../../fixtures/village.json", import.meta.url), "utf8"),
);
const CAMERA = village.presentation.camera;
const VIEWPORT = { width: 1920, height: 1080 };
const HIDE_HUD = ".ro-unit, [data-testid=battle-panel] { display: none !important; }";

/** The camera curve's pitch at `distance` (the controller's own rule). */
function curvePitch(distance) {
  const curve = CAMERA.pitch_curve;
  if (distance <= curve[0][0]) return curve[0][1];
  for (let k = 1; k < curve.length; k++)
    if (distance <= curve[k][0]) {
      const [[d0, p0], [d1, p1]] = [curve[k - 1], curve[k]];
      return p0 + ((p1 - p0) * (distance - d0)) / (d1 - d0);
    }
  return curve.at(-1)[1];
}

/** The first blue soldier out on his lean in `o`: his tucked-in place and lean. */
function leaning(o) {
  for (const u of o.own)
    for (let k = 0; k < u.memberLeans.length; k++) {
      const lean = u.memberLeans[k];
      if (lean) return { id: u.memberIds[k], at: u.members[k], lean };
    }
  return null;
}

export async function run(ctx) {
  const page = await ctx.newPage({ viewport: VIEWPORT });
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  await page.waitForFunction(() => window.__lab.stats?.().grass.enabled, undefined, {
    timeout: 30000,
  });
  await lab(page, () => window.__lab.route.pause());
  // Deterministic: fixed seed, no player. Step until a soldier leans out.
  let o = await obs(page);
  let man = leaning(o);
  for (let k = 0; k < 240 && !man; k++) {
    await advance(page, 5);
    o = await obs(page);
    man = leaning(o);
  }
  ctx.check(
    "a blue soldier in the wood leans out to fire",
    !!man,
    JSON.stringify({ tick: o.tick, man }),
  );
  if (!man) return page.close();
  // Cameras on him: side on to his line of fire at the ground camera and
  // close, and close behind him (from further behind, the canopy hides
  // everything).
  const enemy = o.identified[0]?.position ?? [man.lean.at[0] + 40, man.lean.at[1]];
  const facing = Math.atan2(enemy[1] - man.at[1], enemy[0] - man.at[0]);
  const target = [(man.at[0] + man.lean.at[0]) / 2, (man.at[1] + man.lean.at[1]) / 2];
  const cameras = {
    // Side on to his line of fire: the enemy to the left, his cover between.
    ground: {
      distance: CAMERA.zoom_min,
      pitch: curvePitch(CAMERA.zoom_min),
      yaw: facing + Math.PI / 2,
    },
    side: { distance: 9, pitch: 0.12, yaw: facing + Math.PI / 2 },
    // Behind him, looking the way he fires: his sideways step past the tree.
    behind: { distance: 7, pitch: 0.18, yaw: facing + Math.PI },
  };
  await page.addStyleTag({ content: HIDE_HUD });
  const shots = async (moment) => {
    for (const [name, c] of Object.entries(cameras)) {
      await lab(
        page,
        (c) =>
          window.__lab.setCamera({
            ...window.__lab.camera(),
            distance: c.distance,
            pitch: c.pitch,
            yaw: c.yaw,
            target: [c.at[0], c.at[1], window.__lab.route.surfaceZ(c.at[0], c.at[1])],
          }),
        { ...c, at: target },
      );
      await lab(page, () => window.__lab.setFrameView("world"));
      await snapshot(ctx, page, `${moment}-${name}-1920x1080.png`);
      await lab(page, () => window.__lab.setFrameView("final"));
    }
  };
  // Play on tick by tick, drawing each, so the poses ease as they would
  // live (a paused battle drawn once holds a crossfade mid-way).
  const play = async (n) => {
    for (let k = 0; k < n; k++) {
      await advance(page, 1);
      await lab(page, () => window.__lab.frame());
    }
  };
  // Let the slide out play (the pose driver eases it) before the shots.
  await play(12);
  await shots("lean-out");
  const out = await obs(page);
  // On until he tucks back in (at most ten seconds).
  let tucked = null;
  for (let k = 0; k < 60 && !tucked; k++) {
    await advance(page, 5);
    const now = await obs(page);
    const u = now.own.find((u) => u.memberIds.includes(man.id));
    const i = u?.memberIds.indexOf(man.id) ?? -1;
    if (i >= 0 && !u.memberLeans[i]) tucked = now;
  }
  if (tucked) {
    await play(15);
    await shots("tucked");
  }
  await ctx.writeEvidence("lean.json", {
    tick: o.tick,
    man,
    facing,
    target,
    cameras,
    stillLeaning: !!leaning(out),
    tuckedAt: tucked?.tick ?? null,
  });
  ctx.check(
    "he tucks back in once his burst is out",
    !!tucked,
    JSON.stringify({ tuckedAt: tucked?.tick ?? null }),
  );
  await page.close();
}
