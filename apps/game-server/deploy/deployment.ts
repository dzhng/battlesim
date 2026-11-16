import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { sameBuild } from "../../../packages/multiplayer/src/build.ts";
import type { BuildIdentity } from "../../../packages/multiplayer/src/wire.ts";
import server from "../../battle-lab/src/online/server.json";
import { DeploymentRefusal, prepareDeployment, readSiteBuild } from "./prepare.ts";
import { describePush, push } from "./push.ts";

/** A read-only check: no browser credential, player name or presence is made. */
export async function lobbyInStep(build: BuildIdentity, request = fetch): Promise<boolean> {
  try {
    const response = await request(new URL("/build", server.address), {
      redirect: "error",
      signal: AbortSignal.timeout(10_000),
    });
    return response.ok && sameBuild(await response.json(), build);
  } catch {
    return false;
  }
}

export type DeploymentOptions = {
  site: string;
  commit?: string;
  push?: boolean;
  check?: boolean;
};

/** Both the CLI and promotion enter here; only this boundary knows the host. */
export async function deployServer(
  options: DeploymentOptions,
  {
    root = fileURLToPath(new URL("../../../", import.meta.url)),
    report = console.log,
    request = fetch,
  } = {},
): Promise<number> {
  try {
    if (options.check) {
      const { build } = await readSiteBuild(options.site, options.commit);
      if (await lobbyInStep(build, request)) {
        report("Lobby is in step with the site; no deploy needed.");
        return 0;
      }
      report(`Lobby needs a deploy for site commit ${build.commit}.`);
      return 2;
    }
    const folder = await prepareDeployment(root, options.site, report, options.commit);
    report(`Prepared folder: ${folder.slice(root.length)}`);
    if (options.push) {
      await push(folder);
      const build = JSON.parse(await readFile(join(folder, "build.json"), "utf8"));
      report(`Lobby deployed for site commit ${build.commit}.`);
    } else describePush(report);
    return 0;
  } catch (error) {
    report(
      error instanceof DeploymentRefusal
        ? error.message
        : "Deployment refused; site identity or local source could not be verified.",
    );
    return 1;
  }
}
