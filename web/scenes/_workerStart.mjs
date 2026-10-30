/** Arm one real worker start to pause, optionally publishing a fixed opening. */
export function installWorkerStartFence() {
  let openingTicks = null;
  const post = Worker.prototype.postMessage;
  Worker.prototype.postMessage = function (message, ...args) {
    if (openingTicks !== null && message.type === "start") {
      const ticks = openingTicks;
      openingTicks = null;
      post.call(this, { type: "pause" });
      const result = post.call(this, message, ...args);
      // A plain lab needs its first observation before its viewport is ready.
      // Negative ids cannot consume the client's ordinary advance waiters.
      if (ticks > 0) post.call(this, { type: "advance", id: -1, ticks });
      return result;
    }
    return post.call(this, message, ...args);
  };
  window.__pauseNextWorkerStart = (ticks = 0) => {
    openingTicks = ticks;
  };
}
