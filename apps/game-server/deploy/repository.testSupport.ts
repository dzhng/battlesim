import { execFileSync } from "node:child_process";
import { cp, mkdir, mkdtemp, rm, symlink } from "node:fs/promises";
import { dirname, join } from "node:path";

/** A tiny local repository with this implementation, then a prose-only commit. */
export async function releaseRepository(root: string) {
  const folder = await mkdtemp(join(root, "throwaway/multiplayer/repository-"));
  const git = (...args: string[]) =>
    execFileSync("git", ["-c", "gc.auto=0", "-c", "maintenance.auto=false", ...args], {
      cwd: folder,
      encoding: "utf8",
    }).trim();
  try {
    const paths = ["apps/game-server", "packages", "web/src", "fixtures"];
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
        "HEAD",
        ...paths,
      ],
      { cwd: root, maxBuffer: 128 * 1024 * 1024 },
    );
    execFileSync("tar", ["-xf", "-", "-C", folder], { input: archive });
    const files = execFileSync(
      "git",
      [
        "ls-files",
        "--cached",
        "--others",
        "--exclude-standard",
        "--",
        "apps/game-server",
        "packages/multiplayer",
      ],
      { cwd: root, encoding: "utf8" },
    )
      .trim()
      .split("\n");
    for (const file of files) {
      await mkdir(dirname(join(folder, file)), { recursive: true });
      try {
        await cp(join(root, file), join(folder, file));
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
        await rm(join(folder, file), { force: true });
      }
    }
    git("init", "--quiet");
    git("add", ".");
    const commit = (message: string) =>
      git(
        "-c",
        "user.name=Release test",
        "-c",
        "user.email=release@example.invalid",
        "-c",
        "core.hooksPath=/dev/null",
        "commit",
        "--quiet",
        "-m",
        message,
      );
    commit("shared build machinery");
    const sourceCommit = git("rev-parse", "HEAD");
    const { writeFile } = await import("node:fs/promises");
    await writeFile(join(folder, "README.md"), "A page-only change.\n");
    git("add", "README.md");
    commit("page prose");
    await cp(join(root, "web/src/wasm"), join(folder, "web/src/wasm"), { recursive: true });
    await symlink(join(root, "web/node_modules"), join(folder, "web/node_modules"), "dir");
    return {
      folder,
      sourceCommit,
      head: git("rev-parse", "HEAD"),
      close: () => rm(folder, { recursive: true, force: true }),
    };
  } catch (error) {
    await rm(folder, { recursive: true, force: true });
    throw error;
  }
}
