// The paid steps: draw a soldier body's input image, generate its mesh, and
// generate an airframe skin. Each logs every call to the ledger before and
// after paying, resumes a request the ledger shows unresolved instead of
// paying again, and keeps every candidate: a soldier's files are named by the
// request that made them (`<key>-<req8>.glb`, `<key>-front-<req8>.png`).
//
// `ctx` is {repo, exec, fetch, wait, every, deadline, now, log}; see
// `../skin-generate.mjs` for the real one.
import { copyFile, mkdir, mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, relative, resolve } from "node:path";

import { draw, fal } from "./fal.mjs";
import { AIRFRAME, GENERATORS, IMAGE } from "./generators.mjs";
import { append, read, unresolved } from "./journal.mjs";
import { pinOf, sha256, writePin } from "./pins.mjs";
import { armies, compose, parseKey } from "./prompt.mjs";

const skins = (family) => `assets/source/skins/${family}`;
const at = (ctx) => (ctx.now?.() ?? new Date()).toISOString();

// Run `step` for each key, `concurrency` at a time; every key runs, and the
// failures are thrown together at the end.
async function each(keys, concurrency, step) {
  const failures = [];
  const queue = [...keys];
  const worker = async () => {
    for (let key; (key = queue.shift()) !== undefined; ) {
      try {
        await step(key);
      } catch (error) {
        failures.push(`${key}: ${error.message}`);
      }
    }
  };
  await Promise.all(Array.from({ length: Math.min(concurrency, keys.length) }, worker));
  if (failures.length) throw new Error(failures.join("\n"));
}

function refusePinned(ctx, family, key, again) {
  if (!again && pinOf(ctx.repo, family, key))
    throw new Error(`${key} already has a pin; pass --again to pay for another candidate`);
}

/** Draw the input image of each soldier body `key` ($0.04 each). */
export async function image(ctx, family, keys, { again = false, concurrency = 6 } = {}) {
  return each(keys, concurrency, async (key) => {
    const { army, look, kind } = parseKey(key, armies(ctx.repo));
    refusePinned(ctx, family, key, again);
    const prompt = compose(army, look, kind, armies(ctx.repo));
    const line = {
      family,
      key,
      step: "image",
      endpoint: IMAGE.model,
      options: { prompt, size: IMAGE.size },
      price_source: "listed",
    };
    await mkdir(join(ctx.repo, skins(family)), { recursive: true });
    const scratch = await mkdtemp(join(tmpdir(), "skin-image-"));
    try {
      const drawn = join(scratch, "front.png");
      try {
        await draw(ctx.exec, IMAGE.model, IMAGE.size, prompt, drawn);
      } catch (error) {
        append(ctx.repo, { ...line, at: at(ctx), status: "failed", price_usd: 0 });
        throw error;
      }
      // The gateway's CLI reports no request id, so an image is known by its bytes.
      const id = sha256(drawn);
      const output = `${skins(family)}/${key}-front-${id.slice(0, 8)}.png`;
      await copyFile(drawn, join(ctx.repo, output));
      append(ctx.repo, {
        ...line,
        at: at(ctx),
        request_id: id,
        status: "completed",
        price_usd: IMAGE.price_usd,
        output,
      });
      ctx.log(`${key}: drew ${output}`);
    } finally {
      await rm(scratch, { recursive: true, force: true });
    }
  });
}

// The input image for `key`: `--image` (copied beside the skins if it lives
// elsewhere), or the key's latest drawing in the ledger.
async function inputFor(ctx, family, key, given) {
  if (given) {
    const path = resolve(given);
    const inside = relative(join(ctx.repo, skins(family)), path);
    if (!inside.startsWith("..")) return relative(ctx.repo, path);
    const placed = `${skins(family)}/${key}-front-${sha256(path).slice(0, 8)}.png`;
    await copyFile(path, join(ctx.repo, placed));
    return placed;
  }
  const drawn = read(ctx.repo).findLast(
    (l) => l.family === family && l.key === key && l.step === "image" && l.status === "completed",
  );
  if (!drawn)
    throw new Error(`no input image for ${key}: run image ${family} ${key}, or pass --image`);
  return drawn.output;
}

const pending = (ctx, family, key) =>
  unresolved(read(ctx.repo)).find(
    (l) => l.family === family && l.key === key && l.step === "generate",
  );

/** Generate each soldier body `key` from its input image; a key with an
 * unresolved request resumes it instead. */
export async function generate(
  ctx,
  family,
  keys,
  { image: given, generator = "tripo", again = false, concurrency = 6 } = {},
) {
  if (given && keys.length > 1) throw new Error("--image names one key's input");
  const settings = GENERATORS[generator];
  if (!settings)
    throw new Error(`no generator ${generator}: ${Object.keys(GENERATORS).join(" or ")}`);
  const transport = fal(ctx);
  return each(keys, concurrency, async (key) => {
    parseKey(key, armies(ctx.repo));
    const candidate = (id) => `${skins(family)}/${key}-${id.slice(0, 8)}.glb`;
    const open = pending(ctx, family, key);
    if (open) {
      ctx.log(`${key}: resuming ${open.request_id}, paying nothing`);
      return finish(ctx, transport, open, candidate(open.request_id));
    }
    refusePinned(ctx, family, key, again);
    const input = await inputFor(ctx, family, key, given);
    const image_url = await transport.upload(join(ctx.repo, input));
    const { request_id, charge } = await transport.submit(settings.endpoint, {
      image_url,
      ...settings.options,
    });
    const line = append(ctx.repo, {
      at: at(ctx),
      family,
      key,
      step: "generate",
      endpoint: settings.endpoint,
      options: settings.options,
      inputs: { [input]: sha256(join(ctx.repo, input)) },
      request_id,
      status: "submitted",
      price_usd: settings.price_usd,
      price_source: "listed",
    });
    ctx.log(`${key}: submitted ${request_id} to ${settings.endpoint}`);
    return finish(ctx, transport, line, candidate(request_id), charge);
  });
}

// Wait for a submitted request, download its model to `output`, and log how it
// ended. A request still running at the deadline stays unresolved.
async function finish(ctx, transport, line, output, charge = null) {
  const { endpoint, request_id: id, key } = line;
  const tag = `${key} ${id.slice(0, 8)}`;
  const status = await transport.poll(endpoint, id, {
    every: ctx.every,
    deadline: ctx.deadline,
    onStatus: (s) => ctx.log(`${tag} ${s}`),
  });
  if (!status) throw new Error(`${tag} is still running; run the same command again to resume it`);
  const end = (fields) => append(ctx.repo, { ...line, at: at(ctx), output: null, ...fields });
  if (status.status !== "COMPLETED") {
    end({ status: "failed", price_usd: 0 });
    throw new Error(
      `${tag} ${status.status}${status.error ? `: ${JSON.stringify(status.error)}` : ""}`,
    );
  }
  const result = await transport.result(endpoint, id);
  const glb = result.model_glb?.url ?? result.model_mesh?.url ?? result.model_urls?.glb?.url;
  if (!glb) {
    end({ status: "failed" });
    throw new Error(`${tag} completed without a GLB: ${JSON.stringify(result).slice(0, 300)}`);
  }
  await transport.download(glb, join(ctx.repo, output));
  const preview = result.rendered_image?.url ?? result.thumbnail?.url;
  if (preview)
    await transport.download(preview, join(ctx.repo, output.replace(/\.glb$/, "-preview.png")));
  const reported = charge > 0;
  const done = end({
    status: "completed",
    price_usd: reported ? charge : line.price_usd,
    price_source: reported ? "reported" : "listed",
    output,
  });
  ctx.log(`${tag}: wrote ${output}`);
  return done;
}

/** An airframe skin from four guide views (`blender/skin_views.py`), pinned
 * as `<appearance>.glb`, on the shared transport. */
export async function airframe(ctx, family, appearance, views, note) {
  const dir = skins(family);
  const glb = `${dir}/${appearance}.glb`;
  const transport = fal(ctx);
  let line = pending(ctx, family, appearance);
  let charge = null;
  if (line) ctx.log(`${appearance}: resuming ${line.request_id}, paying nothing`);
  else {
    await mkdir(join(ctx.repo, dir), { recursive: true });
    const input = { ...AIRFRAME.options };
    const inputs = {};
    for (const [view, field] of Object.entries(AIRFRAME.views)) {
      const path = `${dir}/${appearance}-${view}.png`;
      await copyFile(join(resolve(views), `${view}.png`), join(ctx.repo, path));
      input[field] =
        `data:image/png;base64,${(await readFile(join(ctx.repo, path))).toString("base64")}`;
      inputs[path] = sha256(join(ctx.repo, path));
    }
    const submitted = await transport.submit(AIRFRAME.endpoint, input);
    const { request_id } = submitted;
    charge = submitted.charge;
    line = append(ctx.repo, {
      at: at(ctx),
      family,
      key: appearance,
      step: "generate",
      endpoint: AIRFRAME.endpoint,
      options: AIRFRAME.options,
      inputs,
      request_id,
      status: "submitted",
      price_usd: AIRFRAME.price_usd,
      price_source: "listed",
    });
    ctx.log(`queued ${request_id}`);
  }
  const done = await finish(ctx, transport, line, glb, charge);
  writePin(
    ctx.repo,
    family,
    {
      glb,
      sha256: sha256(join(ctx.repo, glb)),
      generator: AIRFRAME.generator,
      request_id: done.request_id,
      generated: done.at.slice(0, 10),
      terms: AIRFRAME.terms,
      inputs: done.inputs,
      note,
    },
    (p) => p.glb === glb,
  );
  ctx.log(`wrote ${glb} and its pin`);
}
