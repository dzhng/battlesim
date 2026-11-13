// One validator finding, and how a measurement prints in one.

import type { Finding } from "./schema.ts";

/** An error-severity finding. */
export const finding = (code: Finding["code"], message: string, fix: string): Finding => ({
  code,
  severity: "error",
  message,
  fix,
});

/** A measure in metres as a finding prints it, to the millimetre. */
export const fmt = (n: number) => n.toFixed(3);
