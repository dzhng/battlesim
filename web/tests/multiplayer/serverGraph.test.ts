// @vitest-environment node
import { execFileSync } from "node:child_process";
import { mkdtempSync, readdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect, test } from "vitest";

const root = join(import.meta.dirname, "../../..");
const entries = [
  "packages/multiplayer/src/wire.ts",
  "packages/multiplayer/src/link.ts",
  "packages/multiplayer/src/stream/index.ts",
  "packages/multiplayer/src/room.ts",
  "packages/multiplayer/src/frontline.ts",
  "packages/multiplayer/src/setup.ts",
  "packages/skirmish-setup/src/seats.ts",
  "packages/skirmish-setup/src/lobby.ts",
  "apps/game-server/src/battleHost/index.ts",
  "apps/game-server/src/battleHost/worker.ts",
  "apps/game-server/src/serverData.ts",
  "apps/game-server/src/soak.ts",
  "apps/game-server/src/http/index.ts",
  "apps/game-server/src/main.ts",
];

test("multiplayer packages bundle for Node without browser aliases or asset loaders", () => {
  const outdir = mkdtempSync(join(tmpdir(), "battlegame-multiplayer-"));
  try {
    execFileSync(
      "bun",
      ["build", "--target=node", "--format=esm", "--outdir", outdir, ...entries],
      { cwd: root, stdio: "pipe" },
    );
    const output = readdirSync(outdir, { recursive: true })
      .map((name) => join(outdir, name.toString()))
      .filter((path) => statSync(path).isFile())
      .map((path) => readFileSync(path, "utf8"))
      .join("\n");
    expect(output).not.toMatch(/(?:^|[^\w])@(?:packages|apps|web|fixtures)\//);
    expect(output).not.toMatch(/\?(?:raw|url)(?:[^\w]|$)/);
  } finally {
    rmSync(outdir, { recursive: true, force: true });
  }
});
