/** Explicit file boundary for ScenarioBundle downloads/uploads.
 *
 * This module transports canonical text only. The Rust contract/WASM validator
 * remains the owner of bundle semantics, hashes, topology, and asset bytes;
 * keeping this boundary opaque prevents a second browser schema from drifting.
 */
export const SCENARIO_BUNDLE_MAX_BYTES = 32 * 1024 * 1024;

export function scenarioBundleBlob(canonicalJson: string): Blob {
  const bytes = new TextEncoder().encode(canonicalJson);
  if (bytes.byteLength > SCENARIO_BUNDLE_MAX_BYTES)
    throw new Error("scenario bundle exceeds the 32 MiB limit");
  return new Blob([bytes], { type: "application/json" });
}

export async function readScenarioBundle(file: Blob): Promise<string> {
  if (file.size > SCENARIO_BUNDLE_MAX_BYTES)
    throw new Error("scenario bundle exceeds the 32 MiB limit");
  return new TextDecoder("utf-8", { fatal: true }).decode(await file.arrayBuffer());
}

export function downloadScenarioBundle(canonicalJson: string, filename = "scenario.json"): void {
  const url = URL.createObjectURL(scenarioBundleBlob(canonicalJson));
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  // Let Chromium finish consuming the object URL before releasing it. A
  // microtask can cancel the download before Playwright or the browser has
  // opened its stream.
  setTimeout(() => URL.revokeObjectURL(url), 60_000);
}

export interface ScenarioBundleIdentity {
  sha256: string;
  map_hash: string;
  rules_hash: string;
}

export interface ScenarioBundleWasm {
  check_scenario_bundle(bundle: string): string;
  canonical_scenario_bundle(bundle: string): string;
  /** Draft-only admission: derives hashes, then returns canonical bytes.
   * Uploaded files use check/canonical directly and never this method. */
  author_scenario_bundle?(bundle: string): string;
}

export class ScenarioBundleRefused extends Error {
  constructor(
    readonly diagnostics: readonly { code: string; location: string; message: string }[],
  ) {
    super(diagnostics.map((diagnostic) => diagnostic.message).join("; "));
  }
}

/** Validate and return the identity of an uploaded bundle through the shared Rust owner. */
export function validateScenarioBundle(
  wasm: ScenarioBundleWasm,
  bundle: string,
): ScenarioBundleIdentity {
  const outcome = JSON.parse(wasm.check_scenario_bundle(bundle)) as
    | { status: "ok"; identity: ScenarioBundleIdentity }
    | { status: "error"; diagnostics: { code: string; location: string; message: string }[] };
  if (outcome.status === "error") throw new ScenarioBundleRefused(outcome.diagnostics);
  return outcome.identity;
}

/** Canonicalize a builder draft only after the Rust validator has admitted it. */
export function canonicalScenarioBundle(wasm: ScenarioBundleWasm, bundle: string): string {
  validateScenarioBundle(wasm, bundle);
  return wasm.canonical_scenario_bundle(bundle);
}
