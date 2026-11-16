import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { bundleServer } from "./bundle.mjs";
const root = fileURLToPath(new URL("../../", import.meta.url));
const commit = execFileSync("git", ["rev-parse", "HEAD"], { cwd: root, encoding: "utf8" }).trim();
await bundleServer(root, `${root}/throwaway/game-server`, commit);
