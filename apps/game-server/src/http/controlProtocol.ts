/** Private control requests carry the per-battle credential only in this header. */
export const CONTROL_HEADER = "X-Battle-Key";
/** A running battle stops after its lobby no longer renews this lease. */
export const LOBBY_LEASE_MS = 10_000;
/** Preparation may take longer than a running battle's lease. Its live start
 * request holds the lease, bounded by the same deadline at both ends. */
export const CONTROL_REQUEST_TIMEOUT_MS = 120_000;
