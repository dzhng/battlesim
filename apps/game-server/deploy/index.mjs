import { parseArgs } from "node:util";
import { deployServer } from "./deployment.ts";

if (import.meta.main) {
  try {
    const { values } = parseArgs({
      options: {
        site: { type: "string" },
        commit: { type: "string" },
        push: { type: "boolean" },
        check: { type: "boolean" },
      },
    });
    if (!values.site || [values.push, values.check].filter(Boolean).length > 1)
      throw new Error(
        "Usage: bun run deploy:server --site <site-url> [--commit <sha>] [--push | --check]",
      );
    process.exitCode = await deployServer(values);
  } catch {
    console.error(
      "Usage: bun run deploy:server --site <site-url> [--commit <sha>] [--push | --check]",
    );
    process.exitCode = 1;
  }
}
