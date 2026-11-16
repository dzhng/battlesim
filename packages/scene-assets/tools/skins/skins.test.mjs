// The generation tool's money logic against a fake Bento: retries, failures,
// resuming a paid request, refusing to pay twice, pinning and composing.
//
//   node --test packages/scene-assets/tools/skins/skins.test.mjs
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import { copyFile, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

import { append, read } from "./journal.mjs";
import { pin, readPins } from "./pins.mjs";
import { armies, compose } from "./prompt.mjs";
import { airframe, generate, image } from "./steps.mjs";

const REPO = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");
const KEY = "usmc_marpat_b_recon";
const REQUEST = "01aaaaaa-0000-7000-8000-000000000001";
const PNG = Buffer.from("front view png");
const GLB = Buffer.from("glTF body");

// A Bento CLI that answers from a script: `statuses` in turn for each poll,
// `fail` the number of transient failures before the generator submit lands.
function fakeBento({
  statuses = ["IN_PROGRESS", "COMPLETED"],
  fail = 0,
  error = "HTTP 503 Service Unavailable",
} = {}) {
  const calls = [];
  const bodies = [];
  let polls = 0;
  const exec = async (args) => {
    calls.push(args);
    const flag = (name) => args[args.indexOf(name) + 1];
    const out = flag("--out");
    if (args[0] === "local") {
      await writeFile(out, PNG);
      return { code: 0, stdout: `${out}\n`, stderr: "" };
    }
    const path = flag("--path");
    let reply;
    if (args[2] === "post" && path === "/storage/upload/initiate") {
      reply = { upload_url: "https://upload.test/put", file_url: "https://cdn.test/front.png" };
    } else if (args[2] === "post") {
      bodies.push(await readFile(flag("--body-file"), "utf8"));
      if (fail-- > 0) return { code: 1, stdout: "", stderr: error };
      reply = { request_id: REQUEST, status: "IN_QUEUE" };
    } else if (path.endsWith("/status")) {
      reply = { status: statuses[Math.min(polls++, statuses.length - 1)] };
    } else {
      reply = {
        model_mesh: { url: "https://cdn.test/model.glb" },
        rendered_image: { url: "https://cdn.test/preview.png" },
      };
    }
    await writeFile(out, JSON.stringify(reply));
    return { code: 0, stdout: '{"x-duet-charge-usd":"0"}', stderr: "" };
  };
  const fetch = async (url, init = {}) => {
    calls.push([init.method ?? "GET", url]);
    return new Response(init.method === "PUT" ? "" : url.endsWith(".glb") ? GLB : PNG);
  };
  const submits = () =>
    calls.filter((a) => a[2] === "post" && !a.includes("/storage/upload/initiate"));
  return { exec, fetch, calls, bodies, submits };
}

async function sandbox(bento) {
  const repo = await mkdtemp(join(tmpdir(), "skin-generate-"));
  const roster = join(repo, "packages/scene-assets/blender/roster");
  await mkdir(roster, { recursive: true });
  await mkdir(join(repo, "packages/scene-assets/blender/skins"), { recursive: true });
  await copyFile(
    join(REPO, "packages/scene-assets/blender/roster/armies.json"),
    join(roster, "armies.json"),
  );
  const ctx = {
    repo,
    exec: bento.exec,
    fetch: bento.fetch,
    wait: async () => {},
    every: 0,
    now: () => new Date("2026-11-15T12:00:00.000Z"),
    log: () => {},
  };
  return { repo, ctx, done: () => rm(repo, { recursive: true, force: true }) };
}

test("a submit the service refused (503) is retried, and the request is paid and logged once", async () => {
  const bento = fakeBento({ fail: 1 });
  const { repo, ctx, done } = await sandbox(bento);
  await image(ctx, "infantry", [KEY]);
  await generate(ctx, "infantry", [KEY]);
  const lines = read(repo);
  assert.deepEqual(
    lines.map((l) => [l.step, l.status]),
    [
      ["image", "completed"],
      ["generate", "submitted"],
      ["generate", "completed"],
    ],
  );
  assert.equal(bento.submits().length, 2, "one refused attempt, one that landed");
  const done_ = lines.at(-1);
  assert.equal(done_.request_id, REQUEST);
  assert.equal(done_.price_source, "listed");
  assert.ok(done_.price_usd > 0);
  assert.deepEqual(await readFile(join(repo, done_.output)), GLB);
  assert.match(done_.output, new RegExp(`${KEY}-${REQUEST.slice(0, 8)}\\.glb$`));
  await done();
});

test("a FAILED generation is logged as failed, costs nothing and pins nothing", async () => {
  const bento = fakeBento({ statuses: ["IN_QUEUE", "FAILED"] });
  const { repo, ctx, done } = await sandbox(bento);
  await image(ctx, "infantry", [KEY]);
  await assert.rejects(generate(ctx, "infantry", [KEY]), /FAILED/);
  const last = read(repo).at(-1);
  assert.equal(last.status, "failed");
  assert.equal(last.price_usd, 0);
  assert.equal(last.output, null);
  assert.deepEqual(readPins(repo, "infantry"), []);
  await done();
});

test("a submit that timed out (504) is not repeated: it may have landed and been charged", async () => {
  const bento = fakeBento({ fail: 1, error: "HTTP 504 Gateway Timeout" });
  const { repo, ctx, done } = await sandbox(bento);
  await image(ctx, "infantry", [KEY]);
  await assert.rejects(generate(ctx, "infantry", [KEY]), /504/);
  assert.equal(bento.submits().length, 1, "no second paid submit");
  assert.deepEqual(readPins(repo, "infantry"), []);
  await done();
});

test("a restart resumes a submitted request: it polls and pays nothing", async () => {
  const bento = fakeBento();
  const { repo, ctx, done } = await sandbox(bento);
  const front = `assets/source/skins/infantry/${KEY}-front-abcdef12.png`;
  await mkdir(join(repo, dirname(front)), { recursive: true });
  await writeFile(join(repo, front), PNG);
  append(repo, {
    at: "2026-11-15T11:00:00.000Z",
    family: "infantry",
    key: KEY,
    step: "generate",
    endpoint: "tripo3d/h3.1/image-to-3d",
    options: {},
    inputs: { [front]: "abc" },
    request_id: REQUEST,
    status: "submitted",
    price_usd: 0.6,
    price_source: "listed",
    output: null,
  });
  await generate(ctx, "infantry", [KEY]);
  assert.equal(bento.submits().length, 0, "nothing was submitted");
  assert.equal(bento.calls.filter((c) => c[0] === "PUT").length, 0, "nothing was uploaded");
  const lines = read(repo);
  assert.equal(lines.length, 2);
  assert.equal(lines[1].status, "completed");
  assert.equal(lines[1].request_id, REQUEST);
  await done();
});

test("generate refuses a key that already has a pin unless --again", async () => {
  const bento = fakeBento();
  const { ctx, done } = await sandbox(bento);
  await image(ctx, "infantry", [KEY]);
  await generate(ctx, "infantry", [KEY]);
  await pin(ctx, "infantry", KEY, REQUEST);
  const before = bento.submits().length;
  await assert.rejects(generate(ctx, "infantry", [KEY]), /--again/);
  assert.equal(bento.submits().length, before, "the refusal paid nothing");
  await generate(ctx, "infantry", [KEY], { again: true });
  assert.equal(bento.submits().length, before + 1);
  await done();
});

test("pin writes the schema skin_pins.py reads, from the ledger", async () => {
  const bento = fakeBento();
  const { repo, ctx, done } = await sandbox(bento);
  await image(ctx, "infantry", [KEY]);
  await generate(ctx, "infantry", [KEY]);
  await pin(ctx, "infantry", KEY, REQUEST, { note: "chosen" });
  const [p] = readPins(repo, "infantry");
  const drawn = read(repo).find((l) => l.step === "image");
  assert.deepEqual(
    { army: p.army, look: p.look, kind: p.kind },
    { army: "usmc_marpat", look: "b", kind: "recon" },
  );
  assert.equal(p.request_id, REQUEST);
  assert.equal(p.note, "chosen");
  assert.equal(p.prompt, compose("usmc_marpat", "b", "recon", armies(repo)));
  assert.equal(p.image_request_id, drawn.request_id);
  assert.equal(p.image_model, drawn.endpoint);
  assert.ok(p.endpoint && p.options && p.front && p.generator && p.terms && p.generated);
  assert.deepEqual(Object.keys(p.inputs), [drawn.output]);
  assert.equal(p.sha256, createHash("sha256").update(GLB).digest("hex"));
  assert.ok(existsSync(join(repo, p.glb)));
  await done();
});

test("compose() is deterministic and orders studio, army, look, kind", () => {
  const data = armies(REPO);
  const text = compose("eastern_emr", "c", "rifle", data);
  assert.equal(text, compose("eastern_emr", "c", "rifle", data));
  const parts = [
    data.studio,
    data.armies.eastern_emr.prompt,
    data.armies.eastern_emr.looks.c,
    data.kinds.rifle.cue,
  ];
  const at = parts.map((part) => text.indexOf(part));
  assert.ok(
    at.every((i) => i >= 0),
    "every part is in the prompt",
  );
  assert.deepEqual(
    [...at].sort((a, b) => a - b),
    at,
    "in order",
  );
  assert.notEqual(text, compose("eastern_emr", "a", "rifle", data));
});

test("an airframe sends each guide view in its own field and pins the result", async () => {
  const bento = fakeBento();
  const { repo, ctx, done } = await sandbox(bento);
  const views = join(repo, "views");
  await mkdir(views);
  for (const view of ["right", "left", "top", "bottom"])
    await writeFile(join(views, `${view}.png`), `${view} guide view`);
  await airframe(ctx, "probe_family", "probe_app", views, "a note");
  const body = JSON.parse(bento.bodies.at(-1));
  const decoded = (field) => Buffer.from(body[field].split(",")[1], "base64").toString();
  assert.equal(decoded("input_image_url"), "right guide view");
  assert.equal(decoded("back_image_url"), "left guide view");
  assert.equal(decoded("top_image_url"), "top guide view");
  assert.equal(decoded("bottom_image_url"), "bottom guide view");
  const [p] = readPins(repo, "probe_family");
  assert.equal(p.note, "a note");
  assert.equal(p.glb, "assets/source/skins/probe_family/probe_app.glb");
  await done();
});
