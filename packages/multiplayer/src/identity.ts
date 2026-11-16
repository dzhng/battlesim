/** Browser credentials travel only in this header, never in a route or a view. */
export const BROWSER_ID_HEADER = "X-Player";
const BROWSER_ID_PATTERN = "[a-f0-9]{32}";
const browserId = new RegExp(`^${BROWSER_ID_PATTERN}$`);

export function isBrowserId(value: unknown): value is string {
  return typeof value === "string" && browserId.test(value);
}

/** Diagnostics cannot retain credentials, including ones already released. */
export function scrubCredentials(message: string): string {
  return message.replace(new RegExp(BROWSER_ID_PATTERN, "gi"), "[credential]");
}
