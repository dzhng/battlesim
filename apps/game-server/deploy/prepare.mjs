import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, readFile, readdir, rm, symlink, writeFile } from "node:fs/promises";
import { dirname, join, relative } from "node:path";
import { bundleServer } from "../bundle.mjs";

async function download(site, path) {
  // Never follow a site redirect into another service, including on a local rehearsal.
  const response = await fetch(new URL(path, site), { redirect: "error" });
  if (!response.ok) throw new Error(`Site refused ${path}: ${response.status}`);
  return new Uint8Array(await response.arrayBuffer());
}
export async function prepareDeployment(root, siteAddress, report) {
  const site = new URL(siteAddress);
  if (!["http:", "https:"].includes(site.protocol)) throw new Error("Site must use HTTP or HTTPS");
  site.pathname = site.pathname.replace(/\/?$/, "/");
  const build = JSON.parse(new TextDecoder().decode(await download(site, "build.json")));
  const commit = execFileSync("git", ["rev-parse", "HEAD"], { cwd: root, encoding: "utf8" }).trim();
  if (build.commit !== commit) throw new Error("Site commit differs from checkout");
  const wasm = await download(site, "sim/game_wasm_bg.wasm");
  const sim = createHash("sha256").update(wasm).digest("hex");
  if (build.sim !== sim) throw new Error("Site simulation hash differs from build.json");
  report(`Verified checkout commit: ${commit}`);
  report(`Verified simulation SHA-256: ${sim}`);
  const glue = await download(site, "sim/game_wasm.js");
  const scratch = join(root, "throwaway/multiplayer/slice29");
  await mkdir(scratch, { recursive: true });
  const staging = await mkdtemp(join(scratch, "deploy-"));
  const source = join(staging, "source");
  const folder = join(staging, "server");
  try {
    await mkdir(source);
    // Archive source without invoking any external smudge filter or fetching art.
    const archive = execFileSync(
      "git",
      [
        "-c",
        "filter.lfs.process=",
        "-c",
        "filter.lfs.smudge=",
        "-c",
        "filter.lfs.required=false",
        "archive",
        commit,
        "apps/game-server/src",
        "apps/game-server/deploy",
        "packages",
        "web/src",
        "fixtures",
      ],
      { cwd: root, maxBuffer: 128 * 1024 * 1024 },
    );
    execFileSync("tar", ["-xf", "-", "-C", source], { input: archive });
    await symlink(join(root, "web/node_modules"), join(source, "web/node_modules"), "dir");
    const simFolder = join(source, "web/src/wasm");
    await mkdir(simFolder, { recursive: true });
    await writeFile(join(simFolder, "game_wasm.js"), glue);
    await writeFile(join(simFolder, "game_wasm_bg.wasm"), wasm);
    // wasm-bindgen's glue imports its panic reporter. Serve and bundle those exact bytes too.
    for (const [, path] of new TextDecoder()
      .decode(glue)
      .matchAll(/from ['"](\.\/snippets\/[A-Za-z0-9_./-]+\.js)['"]/g)) {
      if (path.split("/").includes("..")) throw new Error("Invalid simulation dependency");
      await mkdir(dirname(join(simFolder, path)), { recursive: true });
      await writeFile(join(simFolder, path), await download(site, `sim/${path.slice(2)}`));
    }
    const battleFolder = join(staging, "battle");
    const bundled = await bundleServer(source, battleFolder, commit);
    await mkdir(folder);
    execFileSync("tar", [
      "-czf",
      join(folder, "battle.tar.gz"),
      "-C",
      battleFolder,
      "battle.mjs",
      "worker.mjs",
      "build.json",
      "fixtures",
      "web",
    ]);
    await rm(battleFolder, { recursive: true, force: true });
    await bundleServer(source, folder, commit, "apps/game-server/deploy/lobby.ts");
    // The lobby never hosts a simulation worker or a child battle.
    for (const file of ["main.mjs", "battle.mjs", "worker.mjs", "web", "fixtures"])
      await rm(join(folder, file), { recursive: true, force: true });
    await writeFile(
      join(folder, "settings.json"),
      await readFile(join(source, "apps/game-server/deploy/settings.json")),
    );
    if (bundled.sim !== sim) throw new Error("Bundled simulation differs from site");
    await writeFile(
      join(folder, "package.json"),
      JSON.stringify({ private: true, type: "module", scripts: { start: "node lobby.mjs" } }) +
        "\n",
    );
    report("Verified server bundle and game data extracted from that commit");
    for (const path of (await readdir(folder, { recursive: true, withFileTypes: true })).filter(
      (entry) => entry.isFile(),
    )) {
      const file = join(path.parentPath, path.name);
      report(`Would push: ${relative(folder, file)} (${(await readFile(file)).length} bytes)`);
    }
    return folder;
  } catch (error) {
    await rm(staging, { recursive: true, force: true });
    throw error;
  } finally {
    await rm(source, { recursive: true, force: true });
  }
}
