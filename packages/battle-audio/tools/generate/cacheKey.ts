import { createHash } from "node:crypto";

export const sha256 = (bytes: Uint8Array | string) =>
  createHash("sha256").update(bytes).digest("hex");

/** JSON with sorted object keys, so equal settings always hash alike. */
function canonical(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (value && typeof value === "object")
    return `{${Object.keys(value)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${canonical((value as Record<string, unknown>)[key])}`)
      .join(",")}}`;
  return JSON.stringify(value);
}

/** What a raw master is a function of: changing any of these buys a new take. */
export function generationKey(input: {
  /** The line as spoken, after `{callsign}` expansion. */
  text: string;
  voiceId: string;
  model: string;
  settings: object;
  retake: number;
}): string {
  return sha256(canonical(input));
}

/** What a served file is a function of: changing any of these re-renders locally, for free. */
export function processingKey(input: {
  rawSha: string;
  /** The profile's whole definition, so tuning a constant re-renders. */
  profile: object;
  bedSha: string | null;
  encoder: object;
}): string {
  return sha256(canonical(input));
}
