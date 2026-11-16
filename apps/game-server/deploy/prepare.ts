import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, readFile, readdir, rm, symlink, writeFile } from "node:fs/promises";
import { dirname, join, relative } from "node:path";
import { sameBuild } from "../../../packages/multiplayer/src/build.ts";

export class DeploymentRefusal extends Error {}

async function download(site: URL, path: string) {
  // Never follow a site redirect into another service, including on a local rehearsal.
  const response = await fetch(new URL(path, site), { redirect: "error" });
  if (!response.ok) throw new DeploymentRefusal(`Site refused ${path}: ${response.status}`);
  return new Uint8Array(await response.arrayBuffer());
}
export async function readSiteBuild(siteAddress: string, expectedCommit?: string) {
  const site = new URL(siteAddress);
  if (!["http:", "https:"].includes(site.protocol) || site.username || site.password)
    throw new DeploymentRefusal("Site must be an HTTP or HTTPS address without credentials");
  site.pathname = site.pathname.replace(/\/?$/, "/");
  const build = JSON.parse(new TextDecoder().decode(await download(site, "build.json")));
  if (!/^[a-f0-9]{40}$/.test(build.commit))
    throw new DeploymentRefusal("Site commit does not exist in the local repository");
  if (expectedCommit && build.commit !== expectedCommit)
    throw new DeploymentRefusal("Site does not name the promoted commit");
  return { site, build };
}

export async function prepareDeployment(
  root: string,
  siteAddress: string,
  report: (text: string) => void,
  expectedCommit?: string,
) {
  const { site, build } = await readSiteBuild(siteAddress, expectedCommit);
  let commit: string;
  try {
    commit = execFileSync("git", ["rev-parse", "--verify", `${build.commit}^{commit}`], {
      cwd: root,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    }).trim();
  } catch {
    throw new DeploymentRefusal(
      "Site commit does not exist in the local repository; fetch that commit before deploying",
    );
  }
  const wasm = await download(site, "sim/game_wasm_bg.wasm");
  const sim = createHash("sha256").update(wasm).digest("hex");
  if (build.sim !== sim)
    throw new DeploymentRefusal("Site simulation hash differs from build.json");
  report(`Verified site commit in local repository: ${commit}`);
  report(`Verified simulation SHA-256: ${sim}`);
  const glue = await download(site, "sim/game_wasm.js");
  const scratch = join(root, "throwaway/multiplayer/releases");
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
        "apps/game-server/bundle.mjs",
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
      if (path.split("/").includes(".."))
        throw new DeploymentRefusal("Invalid simulation dependency");
      await mkdir(dirname(join(simFolder, path)), { recursive: true });
      await writeFile(join(simFolder, path), await download(site, `sim/${path.slice(2)}`));
    }
    const battleFolder = join(staging, "battle");
    const bundle = async (out: string, entry?: string) => {
      execFileSync(
        "bun",
        [
          join(source, "apps/game-server/bundle.mjs"),
          source,
          out,
          commit,
          ...(entry ? [entry] : []),
        ],
        { cwd: source, stdio: ["ignore", "pipe", "pipe"] },
      );
      const bundled = JSON.parse(await readFile(join(out, "build.json"), "utf8"));
      if (!sameBuild(bundled, build))
        throw new DeploymentRefusal("Bundled server identity differs from site");
    };
    await bundle(battleFolder);
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
    await bundle(folder, "apps/game-server/deploy/lobby.ts");
    // The lobby never hosts a simulation worker or a child battle.
    for (const file of ["main.mjs", "battle.mjs", "worker.mjs", "web", "fixtures"])
      await rm(join(folder, file), { recursive: true, force: true });
    await writeFile(
      join(folder, "settings.json"),
      await readFile(join(source, "apps/game-server/deploy/settings.json")),
    );
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
