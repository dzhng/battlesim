#!/usr/bin/env bun
// Size budget for source files, so an agent can read any file whole. Tokens
// are estimated as bytes / 4. Run with --help for modes and hook setup.
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import { extname, join, relative } from "node:path";
import { pathToFileURL } from "node:url";
import { parseArgs } from "node:util";

const usage = `Usage:
  check-file-size.mjs --all [--max-tokens N]
      List every tracked source file over the budget, largest first.
  check-file-size.mjs [--max-tokens N] <path>...
      Ratchet the staged versions of <path>: a file over the budget fails
      unless HEAD already had it over the budget and it did not grow. A
      staged rename is measured against the file it came from.

Source is any extension GitHub Linguist assigns to a programming language
and no prose language (.md is not source). Files marked linguist-generated
or linguist-vendored in .gitattributes are skipped. Language data comes
from the linguist-languages package: Bun fetches it in a repository without
node_modules; otherwise add it as a dev dependency.

The default budget is 8,000 tokens. Exit 1 means a file broke the budget.

Pre-commit install (lint-staged), passing every staged file:
  "*": "bun <path-to>/check-file-size.mjs"
Commit this script into the repository and point the hook at that copy, so
every clone runs the same check.`;

const splitRules = `Split a file before adding to it:
  - Split by concept, never by line count: each new file owns one idea and is
    named for it. No part1/part2, utils, helpers, or misc files.
  - Move whole items (types with their impls, functions with the contract
    their callers rely on) and keep the public interface where callers
    already import it.
  - Move tests beside the code they test, grouped the way the code is.
  - Commit the split on its own, before the change that needed the room. A
    split must not change behaviour; prove it with the checks the repository
    already uses for that.`;

const { values, positionals } = parseArgs({
  allowPositionals: true,
  options: {
    all: { type: "boolean" },
    help: { type: "boolean" },
    "max-tokens": { type: "string", default: "8000" },
  },
});
if (values.help) {
  console.log(`${usage}\n\n${splitRules}`);
  process.exit(0);
}
const maxTokens = Number(values["max-tokens"]);
const maxBytes = maxTokens * 4;

const git = (args, input) =>
  execFileSync("git", args, {
    encoding: "utf8",
    input,
    maxBuffer: 1 << 30, // A large repository lists tens of megabytes of paths.
    stdio: ["pipe", "pipe", "ignore"],
  });
const root = git(["rev-parse", "--show-toplevel"]).trim();

// Resolved from the repository, not from wherever this script is installed.
let languages;
try {
  const require = createRequire(join(root, "package.json"));
  languages = await import(pathToFileURL(require.resolve("linguist-languages")).href);
} catch {
  console.error(
    "check-file-size: install linguist-languages as a dev dependency of this repository.",
  );
  process.exit(1);
}
const claims = new Map();
for (const { type, extensions = [] } of Object.values(languages)) {
  for (const extension of extensions) {
    claims.set(extension, [...(claims.get(extension) ?? []), type]);
  }
}
const isSource = (path) => {
  const types = claims.get(extname(path).toLowerCase()) ?? [];
  return types.includes("programming") && !types.includes("prose");
};

// Paths .gitattributes marks as generated or vendored, read from the index.
const unauthored = (paths) => {
  if (paths.length === 0) return new Set();
  const output = git(
    ["check-attr", "--cached", "-z", "--stdin", "linguist-generated", "linguist-vendored"],
    paths.join("\0"),
  ).split("\0");
  const marked = new Set();
  for (let i = 0; i + 2 < output.length; i += 3) {
    if (output[i + 2] === "true" || output[i + 2] === "set") marked.add(output[i]);
  }
  return marked;
};

const blobBytes = (spec) => {
  try {
    return Number(git(["cat-file", "-s", spec]).trim());
  } catch {
    return null; // Not in that tree: a new file.
  }
};
const tokens = (bytes) => `~${Math.round(bytes / 4).toLocaleString("en")} tokens`;
const sources = (paths) => {
  const candidates = paths.filter(isSource);
  const skipped = unauthored(candidates);
  return candidates.filter((path) => !skipped.has(path));
};

if (values.all) {
  const over = sources(git(["ls-files", "-z"]).split("\0").filter(Boolean))
    .map((path) => ({ path, bytes: blobBytes(`:${path}`) }))
    .filter(({ bytes }) => bytes > maxBytes)
    .sort((a, b) => b.bytes - a.bytes);
  for (const { path, bytes } of over) console.log(`${tokens(bytes).padStart(15)}  ${path}`);
  console.log(`Source files over ${maxTokens.toLocaleString("en")} tokens: ${over.length}.`);
  process.exit(over.length > 0 ? 1 : 0);
}

const origin = new Map();
for (const line of git(["diff", "--cached", "--find-renames", "--name-status"]).split("\n")) {
  const [status, from, to] = line.split("\t");
  if (status?.startsWith("R")) origin.set(to, from);
}
const failures = [];
for (const path of sources(positionals.map((path) => relative(root, path)))) {
  const bytes = blobBytes(`:${path}`);
  if (bytes === null || bytes <= maxBytes) continue;
  const before = blobBytes(`HEAD:${origin.get(path) ?? path}`);
  if (before !== null && bytes <= before) continue;
  failures.push({ path, bytes, before });
}
if (failures.length > 0) {
  console.error(
    `Source files may not exceed ${maxTokens.toLocaleString("en")} tokens (bytes / 4).`,
  );
  for (const { path, bytes, before } of failures) {
    const was = before === null ? "new file" : `was ${tokens(before)}, grew`;
    console.error(`  ${path}: ${tokens(bytes)} (${was})`);
  }
  console.error(
    `\nA file already over the budget may stay that size or shrink, but not grow.\n${splitRules}`,
  );
  process.exit(1);
}
