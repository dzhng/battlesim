/** Base names exclude #, which belongs only to the server's clash suffix. */
export const CALLSIGN_LIMIT = 24;

/** Clean instead of refusing edits: page and server accept the same base name. */
export function cleanCallsign(value: string): string {
  return value
    .replace(/[^A-Za-z0-9 _-]/g, "")
    .trim()
    .slice(0, CALLSIGN_LIMIT)
    .trim();
}
