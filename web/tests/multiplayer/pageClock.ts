const flush = () => new Promise<void>((resolve) => setImmediate(resolve));

/** Both time and release callbacks belong to the test, including a suspended tab. */
export class PageClock {
  private time = 10000;
  private tasks = new Set<{ at: number; run: () => void }>();
  now = () => this.time;
  schedule = (run: () => void, delay: number) => {
    const task = { at: this.time + delay, run };
    this.tasks.add(task);
    return () => void this.tasks.delete(task);
  };
  dispose = () => this.tasks.clear();
  /** The OS suspended execution: time passes without any release callback. */
  suspend(ms: number) {
    this.time += ms;
  }
  async advance(ms: number) {
    const end = this.time + ms;
    for (;;) {
      const next = [...this.tasks].sort((a, b) => a.at - b.at)[0];
      if (!next || next.at > end) break;
      this.time = Math.max(this.time, next.at);
      this.tasks.delete(next);
      next.run();
      await flush();
    }
    this.time = end;
    await flush();
  }
}
