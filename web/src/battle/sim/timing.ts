/** Fixed simulation step. The fixture's tick rate is authoritative; this is
 * how many completed ticks the scheduler may run in one wake-up before it
 * sheds wall-clock debt and reports running slow instead of spiralling. */
export const MAX_CATCHUP_TICKS = 4;

/** Publication buffers in flight between producer and consumer. */
export const PUBLICATION_POOL = 2;
