// The temporary host boundary. No command below has been exercised against the host.
import { execFileSync } from "node:child_process";
import { readdir, writeFile } from "node:fs/promises";
import { join, relative } from "node:path";

export function describePush(report) {
  report(
    "Push is OFF. Would upload the listed files with bento file update, then register a private app with bento app create.",
  );
  report("Proxy client-address header is unconfigured: verify it here before P25 relies on it.");
}

/** Explicit --push only. Requires a new app slug; never promotes or makes it public. */
export async function push(folder, app) {
  if (!app || !/^[a-z0-9][a-z0-9-]*$/.test(app))
    throw new Error("--push requires --app <new-app-slug>");
  const files = await readdir(folder, { recursive: true, withFileTypes: true });
  for (const entry of files.filter((file) => file.isFile())) {
    const file = join(entry.parentPath, entry.name);
    execFileSync(
      "bento",
      [
        "file",
        "update",
        "--path",
        `apps/${app}/${relative(folder, file)}`,
        "--in",
        file,
        "--no-auto-upgrade",
      ],
      { stdio: "inherit" },
    );
  }
  const body = join(folder, "..", "app-request.json");
  await writeFile(
    body,
    JSON.stringify({
      path: `apps/${app}`,
      command: "HOST=0.0.0.0 PORT=3000 node main.mjs --dev",
      installCommand: "",
      apiScopes: [],
    }),
  );
  execFileSync(
    "bento",
    ["app", "create", "--slug", app, "--port", "3000", "--body-file", body, "--no-auto-upgrade"],
    { stdio: "inherit" },
  );
}
