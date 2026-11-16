// The temporary host boundary. The runner is injectable; dry runs never invoke it.
import { execFileSync } from "node:child_process";
import { readdir, writeFile } from "node:fs/promises";
import { dirname, join, relative } from "node:path";

export function describePush(report: (text: string) => void) {
  report(
    "Push is OFF. Would create remote folders, upload the listed files, then create or update and restart the app with services access.",
  );
  report(
    "Proxy client-address header is unconfigured: verify the public route before trusting one.",
  );
}
export type CommandRunner = (command: string, args: string[]) => string | Promise<string>;
const run: CommandRunner = (command, args) =>
  execFileSync(command, args, { encoding: "utf8", stdio: ["ignore", "pipe", "inherit"] });

/** Explicit --push only. Workspace comes from the caller, never repository data.
 *  It never changes who can reach the app: making a lobby public is its
 *  operator's own step. */
export async function push(
  folder: string,
  app: string | undefined,
  workspace = process.env.BENTO_WORKSPACE,
  runner: CommandRunner = run,
) {
  if (!app || !/^[a-z0-9][a-z0-9-]*$/.test(app))
    throw new Error("--push requires --app <app-slug>");
  if (!workspace) throw new Error("--push requires --workspace or BENTO_WORKSPACE");
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
  if (!exists) {
    await writeFile(body, JSON.stringify({ path: `apps/${app}`, ...runtime }));
    await command("app", "create", "--slug", app, "--body-file", body);
    return;
  }
  await writeFile(body, JSON.stringify(runtime));
  await command("app", "update", "--app-slug", app, "--body-file", body);
  await command("app", "restart", "--app-slug", app);
}
