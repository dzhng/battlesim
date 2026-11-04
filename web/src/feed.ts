// Large, often-changing presentation data reaches its consumer through one
// stable object rather than as props that
// change with it. React's development build logs every changed prop of a
// commit into a `performance.measure` detail, walking objects two levels
// deep and typed arrays element by element: an overlay mesh (a Float32Array)
// or the fog world's height and foliage grids copied into every detail, tens
// of megabytes a commit once a house fell in the ground lab, until cloning a
// detail ran the page out of memory. The scene runner
// fails any page whose measure details grow that large.
import { useCallback, useLayoutEffect, useState, useSyncExternalStore } from "react";

/** What a consumer reads: the latest value and its changes. */
export interface FeedSource<T> {
  readonly current: T;
  subscribe(listener: (value: T) => void): () => void;
}

export class Feed<T> implements FeedSource<T> {
  private value: T;
  private readonly listeners = new Set<(value: T) => void>();
  constructor(initial: T) {
    this.value = initial;
  }
  get current(): T {
    return this.value;
  }
  set(value: T) {
    this.value = value;
    for (const listener of this.listeners) listener(value);
  }
  /** Called with every new value; returns the unsubscribe. */
  subscribe(listener: (value: T) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }
}

/** One feed for the component's life, carrying the latest `value`. */
export function useFeed<T>(value: T): Feed<T> {
  const [feed] = useState(() => new Feed<T>(value));
  // React subscribers must see the committed publication before the browser paints.
  useLayoutEffect(() => feed.set(value), [feed, value]);
  return feed;
}

/** Render the latest publication without placing its contents in React props. */
export function useFeedValue<T>(feed: FeedSource<T>): T {
  return useSyncExternalStore(
    useCallback((changed) => feed.subscribe(changed), [feed]),
    useCallback(() => feed.current, [feed]),
  );
}
