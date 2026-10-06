/** Fixed simulation step. The fixture's tick rate is authoritative; this is
 * how many completed ticks the scheduler may run in one wake-up before it
 * sheds wall-clock debt and reports running slow instead of spiralling. */
export const MAX_CATCHUP_TICKS = 4;

/** Publication buffers in flight between producer and consumer: enough that
 * a slow frame on the page (the page renders the latest tick once a frame)
 * does not hold the simulation back, few enough that it never runs far ahead
 * of what is drawn. */
export const PUBLICATION_POOL = 3;
