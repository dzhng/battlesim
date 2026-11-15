import { parseArgs } from "node:util";
import { fileURLToPath } from "node:url";
import { prepareDeployment } from "./prepare.mjs";
import { describePush, push } from "./bento.mjs";

try {
  const { values } = parseArgs({
    options: {
      site: { type: "string" },
      push: { type: "boolean", default: false },
      app: { type: "string" },
    },
  });
  if (!values.site)
    throw new Error("Usage: bun run deploy:server --site <site-url> [--push --app <new-app-slug>]");
  const root = fileURLToPath(new URL("../../../", import.meta.url));
  const folder = await prepareDeployment(root, values.site, console.log);
  console.log(`Prepared folder: ${folder.slice(root.length)}`);
  if (values.push) await push(folder, values.app);
  else describePush(console.log);
} catch (error) {
  console.error(error instanceof Error ? error.message : "Deployment refused");
  process.exitCode = 1;
}
