// GPU-backed asset commands. Run through asset.mjs under the repository GPU lock.
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { basename, dirname, join, relative, resolve } from "node:path";
import { parseArgs } from "node:util";
import { lfsPointerOid, lfsPullCommand } from "../packages/scene-assets/src/glb.ts";
import { sheetReferences } from "../packages/scene-assets/src/references.ts";
const ROOT = resolve(dirname(new URL(import.meta.url).pathname), "..");
const REFERENCES = join(ROOT, "assets/references");
const readJson = (path) => JSON.parse(readFileSync(path, "utf8"));
const catalog = () => readJson(join(ROOT, "assets/catalog.json"));
const repoPath = (path) => relative(ROOT, resolve(path));

/** What a page that never became ready says went wrong: its failure panel's
 *  message and details (a missing resource names its file), else its text. */
async function pageFailure(page) {
  const details = page.locator('[data-testid="error"] [aria-expanded="false"]');
  if (await details.count()) await details.first().click();
  return page.evaluate(() => {
    const failure = document.querySelector('[data-testid="error"]');
    if (!failure) return document.body.innerText.trim().slice(0, 600);
    const lines = [...failure.querySelectorAll('[data-testid="error-details"] li')];
    const message = failure.querySelector(".hud-error")?.textContent ?? "";
    return [message, ...lines.map((li) => li.textContent)].join("\n  ");
  });
}

export async function sheet(args) {
  const { values, positionals } = parseArgs({
    args,
    allowPositionals: true,
    options: {
      out: { type: "string" },
      accept: { type: "boolean" },
      unit: { type: "string" },
      type: { type: "string" },
      yaw: { type: "string" },
      clips: { type: "string" },
      side: { type: "string" },
      references: { type: "boolean" },
      variant: { type: "string" },
    },
  });
  const [target] = positionals;
  if (!target)
    throw new Error(
      "sheet <appearance|glb> [--out DIR] [--accept] [--unit U] [--type T] [--yaw DEG] [--clips GLB] [--side blue|red] [--references [--variant V]]",
    );
  const file = target.endsWith(".glb") && existsSync(target) ? target : null;
  const side = values.side ?? "blue";
  if (side !== "blue" && side !== "red") throw new Error(`--side ${side}: blue or red`);
  const name =
    (file ? basename(file, ".glb") : target).replace(/[^\w.-]+/g, "_") +
    (side === "blue" ? "" : `-${side}`);
  const out = resolve(values.out ?? join(ROOT, "throwaway/sheets", name));
  const { startServer, ensureNativeTools } = await import("./scene.mjs");
  const { launchBrowser } = await import("./browser.mjs");
  const { PNG } = await import("pngjs");
  // The workbench loads its unit catalog through the mechanics validator;
  // without it the page never mounts and readiness times out with no cause.
  await ensureNativeTools();
  const server = await startServer();
  const browser = await launchBrowser();
  try {
    const page = await browser.newPage({
      viewport: { width: 1280, height: 800 },
      deviceScaleFactor: 1,
    });
    page.on("pageerror", (e) => console.error(`page: ${e.message}`));
    await page.goto(`${server.url}/workbench`);
    await page
      .waitForFunction(() => window.__lab?.ready && window.__workbench, undefined, {
        timeout: 60000,
      })
      .catch(async (error) => {
        throw new Error(`the workbench never became ready: ${await pageFailure(page)}`, {
          cause: error,
        });
      });
    if (file)
      await page.evaluate(
        ([n, bytes, opts]) =>
          window.__workbench.drop(n, new Uint8Array(bytes), {
            ...opts,
            ...(opts.clips
              ? { clips: { path: opts.clips.path, bytes: new Uint8Array(opts.clips.bytes) } }
              : {}),
          }),
        [
          basename(file),
          Array.from(readFileSync(file)),
          {
            ...(values.unit ? { unit: values.unit } : {}),
            ...(values.type ? { type: values.type } : {}),
            ...(values.yaw !== undefined ? { yaw: Number(values.yaw) } : {}),
            ...(values.clips
              ? {
                  clips: {
                    path: repoPath(values.clips),
                    bytes: Array.from(readFileSync(values.clips)),
                  },
                }
              : {}),
          },
        ],
      );
    else await page.evaluate((n) => window.__workbench.select(n), target);
    await page.evaluate((s) => window.__workbench.setSide(s), side);
    const adapter = await page.evaluate(() => window.__lab.adapter);
    const impostor = await page.evaluate(() => window.__workbench.bakeImpostor());
    const result = await page.evaluate(() => window.__workbench.sheet());
    mkdirSync(out, { recursive: true });
    const png = (dataUrl) => Buffer.from(dataUrl.split(",")[1], "base64");
    writeFileSync(join(out, "contact.png"), png(result.contact));
    for (const strip of result.strips)
      writeFileSync(join(out, `strip-${strip.name}.png`), png(strip.png));
    writeFileSync(join(out, "surface.png"), png(result.surface));
    if (result.textures) writeFileSync(join(out, "textures.png"), png(result.textures));
    for (const layer of ["albedo", "normal"]) {
      const image = new PNG({ width: impostor.width, height: impostor.height });
      Buffer.from(impostor[layer], "base64").copy(image.data);
      writeFileSync(join(out, `impostor-${layer}.png`), PNG.sync.write(image));
      // For eyes: the same atlas over mid grey, since viewers show coverage 0 as white.
      const shown = new PNG({ width: impostor.width, height: impostor.height });
      for (let i = 0; i < image.data.length; i += 4) {
        const a = image.data[i + 3] / 255;
        for (let c = 0; c < 3; c++)
          shown.data[i + c] = Math.round(image.data[i + c] * a + 96 * (1 - a));
        shown.data[i + 3] = 255;
      }
      writeFileSync(join(out, `impostor-${layer}-on-grey.png`), PNG.sync.write(shown));
    }
    writeFileSync(
      join(out, "stats.json"),
      `${JSON.stringify({ ...result.stats, adapter }, null, 2)}\n`,
    );
    console.log(
      `wrote ${out} (contact, ${result.strips.length} strips, stats, impostor ${impostor.hash.slice(0, 12)})`,
    );
    if (values.references) {
      const panel = referenceRows(
        file ? repoPath(file) : catalog().appearances[target]?.source,
        values.variant ?? (file ? basename(file, ".glb") : target),
      );
      if (typeof panel === "string") console.log(panel);
      else {
        const sheet = await page.evaluate(
          ([rows, title]) => window.__workbench.referenceSheet(rows, title),
          [panel.rows, panel.title],
        );
        writeFileSync(join(out, "reference.png"), png(sheet));
        console.log(`wrote ${join(out, "reference.png")} (${panel.title})`);
      }
    }
    const findings = await page.evaluate(() => window.__workbench.state().findings);
    for (const f of findings)
      console.log(`  ${f.severity === "error" ? "ERROR" : "warn "} ${f.code}: ${f.message}`);
    if (values.accept) {
      const review = join(ROOT, "assets/review", name);
      rmSync(review, { recursive: true, force: true });
      cpSync(out, review, { recursive: true });
      console.log(`accepted into ${relative(ROOT, review)}`);
    }
    return 0;
  } finally {
    await browser.close();
    await server.close();
  }
}

/** A roster export receipt can point to a library shared by several source
 * folders. Disabled cards inherit their library from the roster manifest. */
function referenceFamily(source, variant) {
  const folder = source?.match(/^assets\/source\/roster\/(?:infantry\/)?([^/]+)\//)?.[1];
  if (folder !== "disabled") {
    if (!folder) return undefined;
    const receipt = join(ROOT, dirname(source), "source-receipt.json");
    const library = existsSync(receipt) && readJson(receipt).references;
    const path = Array.isArray(library)
      ? library.find((item) => existsSync(join(ROOT, item)) &&
          readJson(join(ROOT, item)).entries.some((entry) => entry.variant === variant))
      : library;
    return path ? basename(dirname(path)) : folder;
  }
  const body = source.replace(/_wreck\.glb$/, ".glb");
  const manifest = readJson(join(ROOT, "fixtures/units/model-manifest.json"));
  return manifest.entries.find((e) => e.source_path === body)?.source_family;
}

function referenceRows(source, variant) {
  const family = referenceFamily(source, variant);
  if (!family) return `no reference panel: ${source ?? variant} is not in a roster family`;
  const folder = join(REFERENCES, family);
  const json = join(folder, "references.json");
  if (!existsSync(json))
    return `no reference panel: assets/references/${family}/references.json does not exist`;
  const library = readJson(json);
  const mime = (file) => (/\.png$/i.test(file) ? "image/png" : "image/jpeg");
  const rows = sheetReferences(library, variant).map(({ view, entry, count }) => {
    if (!entry)
      return {
        view,
        image: null,
        label: `${view}: MISSING, no reference shows it`,
        generated: false,
      };
    const path = join(folder, entry.file);
    const bytes = existsSync(path) ? readFileSync(path) : null;
    if (!bytes || lfsPointerOid(new Uint8Array(bytes)) !== null)
      return {
        view,
        image: null,
        label: `${entry.file}: ${bytes ? `not pulled (${lfsPullCommand(`assets/references/${family}/${entry.file}`)})` : "not in the folder"}`,
        generated: false,
      };
    const credit =
      entry.source === "generated"
        ? `generated by ${entry.model}, seed ${entry.seed}`
        : `${entry.source}, ${entry.author}, ${entry.licence}`;
    return {
      view,
      image: `data:${mime(entry.file)};base64,${bytes.toString("base64")}`,
      label: `${entry.file} · ${credit}${count > 1 ? ` · ${count - 1} more` : ""}`,
      generated: entry.source === "generated",
    };
  });
  return { rows, title: `references: assets/references/${family}, variant ${variant}` };
}
