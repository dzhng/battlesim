/** Yield through a paint before starting the next content-sized fielding job.
 * Two callbacks keep a continuation from joining the callback batch that
 * started this job. Initial fielding can run before a scene exists. */
export function afterFrame(): Promise<void> {
  return new Promise((resolve) =>
    requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
  );
}
