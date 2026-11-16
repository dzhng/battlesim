import { readFile } from "node:fs/promises";
import settings from "./settings.json";

/** A local interpreter of the host CLI's installed endpoint contract. Never executes. */
export function pushStandIn() {
  const commands: { args: string[]; body: unknown }[] = [];
  const directories = new Set();
  const files = new Map<string, string>();
  const apps = new Map<string, Record<string, unknown>>();
  let restarts = 0;
  /** Starts the host will refuse, as it does while an app's files are still arriving. */
  let notReady = 0;
  const runner = async (binary: string, args: string[]) => {
    if (binary !== "bento") throw new Error("Unexpected executable");
    const flag = (name: string) => args[args.indexOf(name) + 1];
    if (!args.includes("--workspace") || flag("--workspace") !== settings.workspace)
      throw new Error("Workspace not supplied");
    const body = args.includes("--body-file")
      ? JSON.parse(await readFile(flag("--body-file"), "utf8"))
      : undefined;
    commands.push({ args, body });
    if (args[0] === "file") {
      if (
        args[1] === "action" &&
        args[2] === "create" &&
        body.operation === "mkdir" &&
        body.recursive
      )
        directories.add(body.path);
      else if (args[1] === "update") {
        const path = flag("--path");
        if (!directories.has(path.slice(0, path.lastIndexOf("/"))))
          throw new Error("Missing parent directory");
        files.set(path, await readFile(flag("--in"), "utf8"));
      } else throw new Error("Unknown file command");
    } else if (args[0] === "app") {
      const action = args[1];
      const slug = flag("--app-slug");
      if (action === "list") return JSON.stringify({ items: [...apps.values()], nextCursor: null });
      if (action === "create") {
        const made = args.includes("--slug") ? flag("--slug") : undefined;
        if (
          !made ||
          apps.has(made) ||
          !directories.has(body.path) ||
          "slug" in body ||
          "port" in body ||
          "visibility" in body ||
          "installCommand" in body
        )
          throw new Error("Invalid registration");
        apps.set(made, { ...body, slug: made, visibility: "private" });
        if (notReady-- > 0) throw new Error("did not become ready");
      } else if (action === "update") {
        if (
          !apps.has(slug) ||
          "path" in body ||
          "slug" in body ||
          "port" in body ||
          "visibility" in body
        )
          throw new Error("Invalid update");
        Object.assign(apps.get(slug)!, body);
      } else if (action === "restart") {
        if (!apps.has(slug)) throw new Error("Missing app");
        if (notReady-- > 0) throw new Error("did not become ready");
        restarts++;
      } else throw new Error("Unknown app command");
    } else throw new Error("Unknown command");
    return "{}";
  };
  return {
    runner,
    files,
    apps,
    commands,
    restarts: () => restarts,
    refuseStarts: (count: number) => {
      notReady = count;
    },
  };
}
