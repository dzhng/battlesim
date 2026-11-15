// Matched look evidence on the real growth route. The open control suppresses
// fog through the existing lab control; it never changes the observed bodies.
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { lab } from "./_lab.mjs";
import { meetStandIns } from "./_stand-in-battle.mjs";

export async function captureLook(
  page,
  unit,
  subject,
  state,
  directory,
  contexts = [
    ["open", true],
    ["fog", false],
  ],
) {
  await mkdir(directory, { recursive: true });
  const metadata = { unit, views: [] };
  try {
    for (const [context, suppressed] of contexts) {
      await lab(page, (on) => window.__lab.suppressFog(on), suppressed);
      for (const [zoom, distance] of [
        ["near", 22],
        ["far", 120],
      ]) {
        await lab(
          page,
          ({ at, distance }) =>
            window.__lab.setCamera({
              ...window.__lab.camera(),
              target: at,
              distance,
              pitch: 0.55,
              yaw: -Math.PI / 2,
            }),
          { at: unit.position, distance },
        );
        await lab(page, () => window.__lab.frame());
        const name = `${subject}-${context}-${zoom}-${state}`;
        await writeFile(join(directory, `${name}.png`), await page.screenshot());
        metadata.views.push(
          await lab(
            page,
            ({ name, at }) => ({
              name,
              camera: window.__lab.camera(),
              presented: window.__lab.route.presented(),
              picks: window.__lab.instances(),
              models: window.__lab.route.drawnModels(),
              fogPoints: Array.from({ length: 25 }, (_, i) => ({
                position: [at[0] + ((i % 5) - 2) * 4, at[1] + (Math.floor(i / 5) - 2) * 4, at[2]],
              })),
            }),
            { name, at: unit.position },
          ),
        );
        if (!suppressed) {
          const view = metadata.views.at(-1);
          view.fogSeen = await lab(
            page,
            async (points) => [...(await window.__lab.fog().probe(points))],
            view.fogPoints,
          );
        }
      }
    }
  } finally {
    await lab(page, () => window.__lab.suppressFog(false));
  }
  await writeFile(
    join(directory, `${subject}-${contexts.map(([name]) => name).join("-")}-${state}.json`),
    JSON.stringify(metadata, null, 2),
  );
}

/** First contact supplies a vehicle at its actual sight boundary, before the
 * later vehicle-and-squad meeting used by the mechanics checks. */
export async function vehicleLook(ctx, directory) {
  const battle = await meetStandIns(ctx, "vehicle");
  const { page, vehicle, timeline } = battle;
  try {
    await captureLook(page, vehicle, "vehicle", "held", directory, [["fog", false]]);
    await battle.release();
    const deadline = Date.now() + 120_000;
    let swapped = false;
    while (!swapped && Date.now() < deadline) {
      await lab(page, () => window.__lab.frame());
      const waiting = await lab(page, () =>
        window.__lab.route
          .drawnModels()
          .some((model) => model.appearance === "__battle_stand_in__"),
      );
      swapped = !waiting && timeline.includes("art committed");
      if (!swapped) await page.waitForTimeout(250);
    }
    ctx.check("first-contact look swaps to its model in the same session", swapped);
    await captureLook(page, vehicle, "vehicle", "model", directory, [["fog", false]]);
  } finally {
    await battle.close();
  }
}
