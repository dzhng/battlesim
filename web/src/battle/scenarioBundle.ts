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
  queueMicrotask(() => URL.revokeObjectURL(url));
}
