/** Requests in flight at once, per kind of call (the Pro account's measured limits). */
export const LIMITS = { tts: 8, stt: 4, music: 2 } as const;

export interface PoolOptions {
  /** Failures in a row, after retries, that stop the pool. */
  maxFailures?: number;
  /** Tries per request, the first included. */
  attempts?: number;
  sleep?: (ms: number) => Promise<void>;
  random?: () => number;
  /** Shared by pools that stop together, so a refused key halts every kind of call. */
  stop?: { reason: Error | null };
}
/** What a failure says about itself: `ApiError` classifies itself, anything else by status. */
interface Classified {
  status?: number;
  fatal?: boolean;
  retryable?: boolean;
}

/**
 * Bounded concurrency with retries. Throttling and server errors back off with
 * full jitter. A fatal error (bad key, exhausted quota) or too many failures in
 * a row stops the pool, and every queued or later request is refused with that
 * reason instead of spending more.
 */
export class Pool {
  private running = 0;
  private failures = 0;
  private readonly waiting: (() => void)[] = [];
  private readonly maxFailures: number;
  private readonly attempts: number;
  private readonly sleep: (ms: number) => Promise<void>;
  private readonly random: () => number;
  private readonly stop: { reason: Error | null };

  constructor(
    private readonly concurrency: number,
    options: PoolOptions = {},
  ) {
    this.maxFailures = options.maxFailures ?? 5;
    this.attempts = options.attempts ?? 5;
    this.sleep = options.sleep ?? ((ms) => new Promise((resolve) => setTimeout(resolve, ms)));
    this.random = options.random ?? Math.random;
    this.stop = options.stop ?? { reason: null };
  }

  get stopped(): Error | null {
    return this.stop.reason;
  }

  async run<T>(task: () => Promise<T>): Promise<T> {
    // A finished request hands its slot straight to the next waiter.
    if (this.running < this.concurrency) this.running++;
    else await new Promise<void>((resolve) => this.waiting.push(resolve));
    try {
      return await this.attempt(task);
    } finally {
      const next = this.waiting.shift();
      if (next) next();
      else this.running--;
    }
  }

  private async attempt<T>(task: () => Promise<T>): Promise<T> {
    for (let attempt = 1; ; attempt++) {
      if (this.stopped) throw this.stopped;
      try {
        const value = await task();
        this.failures = 0;
        return value;
      } catch (error) {
        const kind = error as Classified;
        const status = kind.status ?? 0;
        if (kind.fatal ?? status === 401) {
          this.stop.reason ??= error as Error;
          throw error;
        }
        if ((kind.retryable ?? (status === 429 || status >= 500)) && attempt < this.attempts) {
          await this.sleep(this.random() * Math.min(30_000, 1000 * 2 ** attempt));
          continue;
        }
        if (++this.failures >= this.maxFailures)
          this.stop.reason ??= new Error(`${this.failures} failures in a row; last: ${error}`);
        throw error;
      }
    }
  }
}
