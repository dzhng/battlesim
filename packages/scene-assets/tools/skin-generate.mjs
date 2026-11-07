// Generate an airframe skin (`blender/aircraft_skin.py`) from its four guide
// views (`blender/skin_views.py`), pay for it once and pin it:
//
//   FAL_KEY=... node packages/scene-assets/tools/skin-generate.mjs <family> <appearance> <views_dir> "<note>"
//
// Copies the views to assets/source/skins/<family>/<appearance>-<view>.png,
// runs Hunyuan3D 3.1 Pro (geometry only) on fal with the right side as its
// front, the left as its back, the top and the bottom, writes the GLB beside
// them and replaces the appearance's pin in blender/skins/<family>.json.
// `note` says what the views were drawn from.
import { createHash } from "node:crypto";
import { copyFile, mkdir, readFile, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ENDPOINT = "fal-ai/hunyuan-3d/v3.1/pro/image-to-3d";
const FIELDS = {
  right: "input_image_url",
  left: "back_image_url",
  top: "top_image_url",
  bottom: "bottom_image_url",
};
const repo = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const [family, appearance, views, note] = process.argv.slice(2);
if (!family || !appearance || !views || !note || !process.env.FAL_KEY) {
  console.error(
    'usage: FAL_KEY=... node skin-generate.mjs <family> <appearance> <views_dir> "<note>"',
  );
  process.exit(2);
}
const call = async (url, init = {}) => {
  const res = await fetch(url, {
    ...init,
    headers: { authorization: `Key ${process.env.FAL_KEY}`, "content-type": "application/json" },
  });
  if (!res.ok) throw new Error(`${url}: ${res.status} ${await res.text()}`);
  return res.json();
};
const sha = async (path) =>
  createHash("sha256")
    .update(await readFile(join(repo, path)))
    .digest("hex");

const dir = `assets/source/skins/${family}`;
await mkdir(join(repo, dir), { recursive: true });
const input = { generate_type: "Geometry", face_count: 300000 };
const inputs = {};
for (const [view, field] of Object.entries(FIELDS)) {
  const path = `${dir}/${appearance}-${view}.png`;
  await copyFile(join(resolve(views), `${view}.png`), join(repo, path));
  input[field] = `data:image/png;base64,${(await readFile(join(repo, path))).toString("base64")}`;
  inputs[path] = await sha(path);
}
const job = await call(`https://queue.fal.run/${ENDPOINT}`, {
  method: "POST",
  body: JSON.stringify(input),
});
console.log(`queued ${job.request_id}`);
for (const started = Date.now(); (await call(job.status_url)).status !== "COMPLETED"; ) {
  if (Date.now() - started > 25 * 60_000)
    throw new Error(`${job.request_id} did not finish in 25 minutes`);
  await new Promise((r) => setTimeout(r, 5000));
}
const result = await call(job.response_url);
const glb = `${dir}/${appearance}.glb`;
await writeFile(
  join(repo, glb),
  Buffer.from(
    await (await fetch(result.model_glb?.url ?? result.model_urls?.glb?.url)).arrayBuffer(),
  ),
);

const pinsPath = join(repo, `packages/scene-assets/blender/skins/${family}.json`);
const pins = existsSync(pinsPath)
  ? JSON.parse(await readFile(pinsPath, "utf8")).filter((p) => p.glb !== glb)
  : [];
pins.push({
  glb,
  sha256: await sha(glb),
  generator: `Hunyuan3D 3.1 Pro image to 3D, through fal (${ENDPOINT}), geometry only, 300,000 faces, from four views: the right side as its front, the left side as its back, the top and the bottom`,
  request_id: job.request_id,
  generated: new Date().toISOString().slice(0, 10),
  terms:
    "A paid generation: fal lists the model for commercial use, and its terms leave output content to the customer.",
  inputs,
  note,
});
await writeFile(pinsPath, `${JSON.stringify(pins, null, 2)}\n`);
console.log(`wrote ${glb} and its pin`);
