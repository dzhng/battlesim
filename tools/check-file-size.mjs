// Pre-commit ratchet on source file size, so an agent can read any file whole.
// A file may not be committed over the budget unless it was already over it
// and does not grow. Tokens are estimated as bytes / 4.
import { execFileSync } from "node:child_process";
import { extname, relative } from "node:path";
import * as languages from "linguist-languages";

const maxTokens = 8_000;
const maxBytes = maxTokens * 4;
// Code nobody here authors: vendored libraries and recorded fixtures.
const skipped = [/(^|\/)vendor\//, /(^|\/)fixtures\//];

// Source code is what GitHub Linguist calls a programming language. An
// extension a prose language also claims (.md is Markdown and GCC Machine
// Description) is not source.
const claims = new Map();
for (const { type, extensions = [] } of Object.values(languages)) {
  for (const extension of extensions)
    claims.set(extension, [...(claims.get(extension) ?? []), type]);
}
const isSource = (path) => {
  const types = claims.get(extname(path).toLowerCase()) ?? [];
  return types.includes("programming") && !types.includes("prose");
};

const git = (...args) =>
  execFileSync("git", args, { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] });
const root = git("rev-parse", "--show-toplevel").trim();

// A staged rename or move is measured against the file it came from.
const origin = new Map();
for (const line of git("diff", "--cached", "--find-renames", "--name-status").split("\n")) {
  const [status, from, to] = line.split("\t");
  if (status?.startsWith("R")) origin.set(to, from);
}

const blobBytes = (spec) => {
  try {
    return Number(git("cat-file", "-s", spec).trim());
  } catch {
    return null; // Not in that tree: a new file.
  }
};

const failures = [];
for (const arg of process.argv.slice(2)) {
  const path = relative(root, arg);
  if (!isSource(path) || skipped.some((pattern) => pattern.test(path))) continue;
  const bytes = blobBytes(`:${path}`);
  if (bytes === null || bytes <= maxBytes) continue;
  const before = blobBytes(`HEAD:${origin.get(path) ?? path}`);
  if (before !== null && bytes <= before) continue;
  failures.push({ path, bytes, before });
}

if (failures.length > 0) {
  const tokens = (bytes) => `~${Math.round(bytes / 4).toLocaleString("en")} tokens`;
  console.error(
    `Source files may not exceed ${maxTokens.toLocaleString("en")} tokens (bytes / 4).`,
  );
  for (const { path, bytes, before } of failures) {
    const was = before === null ? "new file" : `was ${tokens(before)}, grew`;
    console.error(`  ${path}: ${tokens(bytes)} (${was})`);
  }
  console.error(`
A file already over the budget may stay that size or shrink, but not grow.
Split it before adding to it:
  - Split by concept, never by line count: each new file owns one idea and is
    named for it. No part1/part2, utils, helpers, or misc files.
  - Move whole items (types with their impls, functions with their callers'
    contract) and keep the public interface where callers already import it.
  - Move tests beside the code they test, grouped the way the code is.
  - Commit the split on its own, before the change that needed the room. A
    split must not change behaviour: battle digests and replays stay unchanged.
  - Use the refactor-clean skill to find the seams.`);
  process.exit(1);
}
