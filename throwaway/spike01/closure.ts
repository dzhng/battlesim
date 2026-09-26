// Import closure over ~/dev/game sources (read-only). Usage:
//   bun closure.ts [--runtime] [--stop=substr,...] [-v] root1.ts dir/ ...
// A root ending in "/" means every .ts file directly in that directory.
import { readFileSync, existsSync, statSync, readdirSync } from "node:fs";
import { dirname, resolve, relative } from "node:path";
const GAME = "/Users/david/dev/game";
const args = process.argv.slice(2);
const runtimeOnly = args.includes("--runtime");
const stop = (args.find((a) => a.startsWith("--stop=")) ?? "--stop=")
  .slice(7)
  .split(",")
  .filter(Boolean);
const verbose = args.includes("-v");
const roots: string[] = [];
for (const r of args.filter((a) => !a.startsWith("-"))) {
  const abs = resolve(GAME, r);
  if (r.endsWith("/")) {
    for (const f of readdirSync(abs)) if (f.endsWith(".ts")) roots.push(resolve(abs, f));
  } else roots.push(abs);
}
const re =
  /(?:^|\n)\s*(import|export)\s+(type\s+)?([^;]*?)\s*from\s*["']([^"']+)["']|import\(\s*["']([^"']+)["']\s*\)|(?:^|\n)\s*import\s+["']([^"']+)["']/g;
function resolveSpec(from: string, spec: string): string | null {
  let base: string;
  if (spec.startsWith(".")) base = resolve(dirname(from), spec);
  else if (spec.startsWith("@packages/")) base = resolve(GAME, "packages", spec.slice(10));
  else return null;
  for (const c of [base, base + ".ts", base + ".tsx", base + "/index.ts"]) {
    if (existsSync(c) && statSync(c).isFile()) return c;
  }
  if (base.endsWith(".js") && existsSync(base.slice(0, -3) + ".ts")) return base.slice(0, -3) + ".ts";
  return null;
}
const seen = new Map<string, number>();
const external = new Map<string, number>();
const importers = new Map<string, Set<string>>();
const q = [...roots];
while (q.length) {
  const f = q.pop()!;
  if (seen.has(f)) continue;
  const src = readFileSync(f, "utf8");
  seen.set(f, src.split("\n").length);
  const rel = relative(GAME, f);
  if (stop.some((s) => rel.includes(s)) && !roots.includes(f)) continue;
  for (const m of src.matchAll(re)) {
    const spec = m[4] ?? m[5] ?? m[6];
    const names = m[3] ?? "";
    const allTypeNames =
      /^\{[^}]*\}$/.test(names.trim()) &&
      names
        .trim()
        .slice(1, -1)
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean)
        .every((s) => s.startsWith("type "));
    const typeOnly = !!m[2] || allTypeNames;
    if (runtimeOnly && typeOnly) continue;
    const r = resolveSpec(f, spec);
    if (!r) {
      external.set(spec, (external.get(spec) ?? 0) + 1);
      continue;
    }
    if (!importers.has(r)) importers.set(r, new Set());
    importers.get(r)!.add(relative(GAME, f));
    q.push(r);
  }
}
const byPkg = new Map<string, { files: number; lines: number }>();
for (const [f, n] of seen) {
  const rel = relative(GAME, f);
  const parts = rel.split("/");
  const pkg = parts[0] === "packages" ? parts.slice(0, 2).join("/") : parts.slice(0, 2).join("/");
  const e = byPkg.get(pkg) ?? { files: 0, lines: 0 };
  e.files++;
  e.lines += n;
  byPkg.set(pkg, e);
}
let tf = 0,
  tl = 0;
for (const [p, e] of [...byPkg].sort()) {
  console.log(`${p.padEnd(34)} ${String(e.files).padStart(4)} files ${String(e.lines).padStart(6)} lines`);
  tf += e.files;
  tl += e.lines;
}
console.log(`${"TOTAL".padEnd(34)} ${String(tf).padStart(4)} files ${String(tl).padStart(6)} lines`);
console.log("external:", [...external.keys()].join(", "));
if (verbose)
  for (const [f, n] of [...seen].sort((a, b) => b[1] - a[1]))
    console.log(
      String(n).padStart(6),
      relative(GAME, f),
      " <- ",
      [...(importers.get(f) ?? [])].slice(0, 3).join(" "),
    );
