/** Private control requests carry the per-battle credential only in this header. */
export const CONTROL_HEADER = "X-Battle-Key";
/** A running battle stops after its lobby no longer renews this lease. */
export const LOBBY_LEASE_MS = 12_000;
/** Preparation may take longer than a running battle's lease. Its live start
 * request holds the lease, bounded by the same deadline at both ends. */
export const CONTROL_REQUEST_TIMEOUT_MS = 120_000;

/** Retry a failed ping inside the silence lease; one stall is not a crash. */
export const CONTROL_PING_EVERY_MS = 1000;
export const CONTROL_PING_TIMEOUT_MS = 4000;
