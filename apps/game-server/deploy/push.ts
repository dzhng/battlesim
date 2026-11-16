// The temporary host boundary. The runner is injectable; dry runs never invoke it.
import { execFileSync } from "node:child_process";
import { readFile, readdir, writeFile } from "node:fs/promises";
import { dirname, join, relative } from "node:path";

export function describePush(report: (text: string) => void) {
  report(
    "Push is OFF. Would create remote folders, upload the listed files, then create or update and restart the app with services access.",
  );
}
export type CommandRunner = (command: string, args: string[]) => string | Promise<string>;
const run: CommandRunner = (command, args) => {
  try {
    return execFileSync(command, args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
  } catch {
    throw new Error("Lobby host request failed");
  }
};

/** Explicit push only. The archived settings own where this release lives.
 *  It never changes who can reach the app: making a lobby public is its
 *  operator's own step. */
export async function push(
  folder: string,
  runner: CommandRunner = run,
  pause: () => Promise<unknown> = () => new Promise((resolve) => setTimeout(resolve, 10_000)),
) {
  const { app, workspace } = JSON.parse(await readFile(join(folder, "settings.json"), "utf8"));
  const command = (...args: string[]) => runner("bento", [...args, "--workspace", workspace]);
  let exists = false;
  let cursor: string | null | undefined;
  do {
    const listed = JSON.parse(
      await command("app", "list", ...(cursor ? ["--cursor", cursor] : [])),
    );
    exists ||= listed.items.some((candidate: { slug: string }) => candidate.slug === app);
    cursor = listed.nextCursor;
  } while (cursor && !exists);
  const entries = await readdir(folder, { recursive: true, withFileTypes: true });
  const files = entries
    .filter((file) => file.isFile())
    .map((entry) => join(entry.parentPath, entry.name))
    .sort();
  const folders = new Set([
    `apps/${app}`,
    ...files.map((file) => dirname(`apps/${app}/${relative(folder, file)}`)),
  ]);
  const body = join(folder, "..", "app-request.json");
  for (const path of [...folders].sort()) {
    await writeFile(body, JSON.stringify({ operation: "mkdir", path, recursive: true }));
    await command("file", "action", "create", "--body-file", body);
  }
  for (const file of files)
    await command(
      "file",
      "update",
      "--path",
      `apps/${app}/${relative(folder, file)}`,
      "--in",
      file,
    );
  const runtime = { command: "node lobby.mjs", apiScopes: ["services"] };
  /** The host starts an app before the files just written have all reached
   *  its machine, and says the app did not come up. It comes up once they
   *  have: ask again a few times before calling the deploy failed. */
  const started = async (start: () => unknown) => {
    for (let attempt = 1; ; attempt++) {
      try {
        await (attempt === 1 ? start() : command("app", "restart", "--app-slug", app));
        return;
      } catch {
        if (attempt === 6) throw new Error("Lobby host could not start the deployment");
        await pause();
      }
    }
  };
  if (!exists) {
    await writeFile(body, JSON.stringify({ path: `apps/${app}`, ...runtime }));
    await started(() => command("app", "create", "--slug", app, "--body-file", body));
    return;
  }
  await writeFile(body, JSON.stringify(runtime));
  await command("app", "update", "--app-slug", app, "--body-file", body);
  await started(() => command("app", "restart", "--app-slug", app));
}
